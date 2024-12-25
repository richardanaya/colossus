// Global state
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
const messagesContainer = document.getElementById("messages");
const captionElement = document.getElementById("caption");
const functionCallsContainer = document.getElementById("functionCalls");
const textInputElement = document.getElementById("textInput");
const sendButton = document.getElementById("sendButton");
const connectButton = document.getElementById("connectButton");
const muteButton = document.getElementById("muteButton");

async function handleSendMessage() {
  const text = textInputElement.value.trim();
  if (!text) return;

  if (dataChannel) {
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
    messages.push({ type: "user", content: text });
    updateMessagesUI();
    textInputElement.value = "";
  }
}

async function createSession() {
  const response = await fetch("/api/sessions", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      model: "gpt-4o-realtime-preview-2024-12-17",
      voice: "alloy",
      instructions: "You are a helpful assistant.",
    }),
  });
  return response.json();
}

async function init() {
  setConnectingState(true);
  try {
    const data = await createSession();
    const EPHEMERAL_KEY = data.client_secret.value;

    const newPc = new RTCPeerConnection();
    pc = newPc;

    const audioEl = document.createElement("audio");
    audioEl.autoplay = true;
    newPc.ontrack = (e) => (audioEl.srcObject = e.streams[0]);

    const dc = newPc.createDataChannel("oai-events");
    dataChannel = dc;

    dc.addEventListener("open", () => {
      setConnectingState(false);
      setConnectedState(true);
      console.log("Connected to OpenAI Realtime API");

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
              description: "Connect two nodes together with specified sockets",
              parameters: {
                type: "object",
                properties: {
                  from_node: {
                    type: "string",
                    description: "Name of the source node",
                  },
                  from_socket: {
                    type: "string",
                    description: "Name of the output socket on the source node",
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
                required: ["from_node", "from_socket", "to_node", "to_socket"],
              },
            },
          ],
          tool_choice: "auto",
        },
      };
      dc.send(JSON.stringify(functionConfig));
    });

    dc.addEventListener("message", handleMessage);

    const ms = await navigator.mediaDevices.getUserMedia({ audio: true });
    const track = ms.getTracks()[0];
    audioTrack = track;
    newPc.addTrack(track);

    const offer = await newPc.createOffer();
    await newPc.setLocalDescription(offer);

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
    await newPc.setRemoteDescription(answer);
  } catch (error) {
    console.error("Failed to initialize:", error);
  } finally {
    setConnectingState(false);
  }
}

function handleMessage(e) {
  const event = JSON.parse(e.data);
  console.log("Received from OpenAI:", event);

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
    updateFunctionCallsUI();
  }

  if (event.type === "response.audio_transcript.delta") {
    caption = event.delta;
    updateCaptionUI();
    const lastMessage = messages[messages.length - 1];
    if (lastMessage?.type === "assistant") {
      lastMessage.content += event.delta;
    } else {
      messages.push({ type: "assistant", content: event.delta });
    }
    updateMessagesUI();
  } else if (event.type === "response.done") {
    caption = "";
    updateCaptionUI();
  }
}

function updateMessagesUI() {
  messagesContainer.innerHTML = messages
    .map(
      (message, index) => `
        <div class="glass-card rounded-lg p-4 mb-4 ${
          message.type === "user" ? "ml-auto" : "mr-auto"
        }" style="max-width: 80%">
            <p class="text-lg">${message.content}</p>
        </div>
    `
    )
    .join("");
  messagesContainer.scrollTop = messagesContainer.scrollHeight;
}

function updateCaptionUI() {
  if (caption) {
    captionElement.textContent = caption;
    captionElement.style.display = "block";
  } else {
    captionElement.style.display = "none";
  }
}

function updateFunctionCallsUI() {
  functionCallsContainer.innerHTML = functionCalls
    .map(
      (call, i) => `
        <div class="glass-card rounded-lg p-4 mb-4">
            <span class="function-name">${call.name}</span>
            <pre class="function-args">${call.args}</pre>
        </div>
    `
    )
    .join("");
}

function setConnectingState(state) {
  isConnecting = state;
  connectButton.disabled = state;
  connectButton.textContent = state
    ? "Connecting..."
    : isConnected
    ? "Connected"
    : "Start Session";
}

function setConnectedState(state) {
  isConnected = state;
  connectButton.textContent = state ? "Connected" : "Start Session";
}

// Event Listeners
connectButton.addEventListener("click", init);

muteButton.addEventListener("click", () => {
  if (audioTrack) {
    isMuted = !isMuted;
    audioTrack.enabled = !isMuted;
    muteButton.textContent = isMuted ? "Unmute" : "Mute";
  }
});

sendButton.addEventListener("click", handleSendMessage);

textInputElement.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    handleSendMessage();
  }
});
