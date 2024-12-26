let isConnecting = false;
let isConnected = false;
let caption = "";
let messages = [];
let functionCalls = [];
let textInput = "";
let dataChannel = null;
let pc = null;
let isMuted = false;
let audioTrack = null;

// DOM Elements
const connectButton = document.getElementById('connectButton');
const muteButton = document.getElementById('muteButton');
const textInputArea = document.getElementById('textInput');
const sendButton = document.getElementById('sendButton');
const messagesContainer = document.getElementById('messages');
const functionCallsContainer = document.getElementById('functionCalls');
const captionElement = document.getElementById('caption');

function updateUI() {
    // Update connect button
    connectButton.textContent = isConnecting ? "Connecting..." : (isConnected ? "Connected" : "Start Session");
    connectButton.disabled = isConnecting;

    // Update mute button
    muteButton.style.display = audioTrack ? 'block' : 'none';
    muteButton.textContent = isMuted ? "Unmute" : "Mute";
    muteButton.disabled = !audioTrack;

    // Update messages
    messagesContainer.innerHTML = messages.map(message => `
        <div class="glass-card" style="margin-bottom: 1rem; ${message.type === 'user' ? 'margin-left: auto;' : 'margin-right: auto;'} max-width: 80%;">
            <p>${message.content}</p>
        </div>
    `).join('');

    // Update caption
    if (caption) {
        captionElement.style.display = 'block';
        captionElement.textContent = caption;
    } else {
        captionElement.style.display = 'none';
    }

    // Update function calls
    functionCallsContainer.innerHTML = functionCalls.map(call => `
        <div class="glass-card">
            <span class="function-name">${call.name}</span>
            <pre class="function-args">${call.args}</pre>
        </div>
    `).join('');
}

async function handleSendMessage() {
    const text = textInputArea.value.trim();
    if (!text || !dataChannel) return;

    console.log("Sending message to OpenAI:", text);
    dataChannel.send(
        JSON.stringify({
            type: "conversation.item.create",
            item: {
                type: "message",
                role: "user",
                content: [
                    {
                        type: "input_text",
                        text: text,
                    },
                ],
            },
        })
    );
    dataChannel.send(
        JSON.stringify({
            type: "response.create",
        })
    );
    
    messages.push({ type: 'user', content: text });
    textInputArea.value = "";
    updateUI();
}

async function init() {
    isConnecting = true;
    updateUI();
    
    try {
        // Get session data from our server
        const response = await fetch('/api/sessions', {
            method: 'POST'
        });
        const data = await response.json();
        const EPHEMERAL_KEY = data.client_secret.value;

        // Create a peer connection
        pc = new RTCPeerConnection();

        // Set up to play remote audio from the model
        const audioEl = document.createElement("audio");
        audioEl.autoplay = true;
        pc.ontrack = (e) => (audioEl.srcObject = e.streams[0]);

        // Configure initial session and set up data channel
        dataChannel = pc.createDataChannel("oai-events");
        dataChannel.addEventListener("open", () => {
            isConnecting = false;
            isConnected = true;
            updateUI();
            console.log("Connected to OpenAI Realtime API");

        // Configure function calls after connection
        const functionConfig = {
          type: "session.update",
          session: {
            tools: [
              {
                type: "function",
                name: "create_node",
                description: "Create a new node in the graph",
                parameters: {
                  type: "object",
                  properties: {
                    name: {
                      type: "string",
                      description: "The unique name for the node",
                    },
                  },
                  required: ["name"],
                },
              },
              {
                type: "function",
                name: "remove_node",
                description: "Remove an existing node from the graph",
                parameters: {
                  type: "object",
                  properties: {
                    name: {
                      type: "string",
                      description: "The name of the node to remove",
                    },
                  },
                  required: ["name"],
                },
              },
              {
                type: "function",
                name: "connect_nodes",
                description:
                  "Connect two nodes together with specified sockets",
                parameters: {
                  type: "object",
                  properties: {
                    from_node: {
                      type: "string",
                      description: "Name of the source node",
                    },
                    from_socket: {
                      type: "string",
                      description:
                        "Name of the output socket on the source node",
                    },
                    to_node: {
                      type: "string",
                      description: "Name of the destination node",
                    },
                    to_socket: {
                      type: "string",
                      description:
                        "Name of the input socket on the destination node",
                    },
                  },
                  required: [
                    "from_node",
                    "from_socket",
                    "to_node",
                    "to_socket",
                  ],
                },
              },
            ],
            tool_choice: "auto",
          },
        };
        dataChannel.send(JSON.stringify(functionConfig));
      });

        // Add local audio track for microphone input
        const ms = await navigator.mediaDevices.getUserMedia({
            audio: true,
        });
        audioTrack = ms.getTracks()[0];
        pc.addTrack(audioTrack);
        updateUI();

        dataChannel.addEventListener("message", (e) => {
            const event = JSON.parse(e.data);

            // Log all incoming messages
            console.log("Received from OpenAI:", event);

            // Handle function calls
            if (event.type === "response.function_call_arguments.delta") {
                console.log("Partial function call:", event.delta);
            } else if (
                event.type === "response.done" &&
                event.response.output?.[0]?.type === "function_call"
            ) {
                const call = event.response.output[0];
                functionCalls.push({
                    name: call.name,
                    args: call.arguments,
                });
                updateUI();
            }

            // Handle audio transcript events
            if (event.type === "response.audio_transcript.delta") {
                caption = event.delta;
                const lastMessage = messages[messages.length - 1];
                if (lastMessage?.type === 'assistant') {
                    lastMessage.content += event.delta;
                } else {
                    messages.push({ type: 'assistant', content: event.delta });
                }
                updateUI();
            } else if (event.type === "response.done") {
                caption = "";
                updateUI();
            }
        });

        // Start the session using the Session Description Protocol (SDP)
        const offer = await pc.createOffer();
        await pc.setLocalDescription(offer);

        const baseUrl = "https://api.openai.com/v1/realtime";
        const model = "gpt-4o-realtime-preview-2024-12-17";
        const sdpResponse = await fetch(`${baseUrl}?model=${model}`, {
            method: "POST",
            body: offer.sdp,
            headers: {
                Authorization: `Bearer ${EPHEMERAL_KEY}`,
                "Content-Type": "application/sdp",
            },
        });

        const answer = {
            type: "answer",
            sdp: await sdpResponse.text(),
        };
        await pc.setRemoteDescription(answer);
    } catch (error) {
        console.error("Failed to initialize:", error);
    } finally {
        isConnecting = false;
        updateUI();
    }
}

// Event Listeners
connectButton.addEventListener('click', init);

muteButton.addEventListener('click', () => {
    if (audioTrack) {
        isMuted = !isMuted;
        audioTrack.enabled = !isMuted;
        updateUI();
    }
});

sendButton.addEventListener('click', handleSendMessage);

textInputArea.addEventListener('keydown', (e) => {
    if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSendMessage();
    }
});

// Initial UI update
updateUI();
