let isConnecting = false;
let isConnected = false;
let currentMode = 'planning'; // Track current mode

// Update mode toggle button
function updateModeToggle() {
  const modeToggle = document.getElementById('modeToggle');
  if (modeToggle) {
    modeToggle.textContent = currentMode === 'planning' ? 'Planning Mode' : 'Developing Mode';
    modeToggle.style.backgroundColor = currentMode === 'planning' ? '#3b82f6' : '#10b981';
  }
}

// Toggle mode between planning and developing
async function toggleMode() {
  try {
    const newMode = currentMode === 'planning' ? 'developing' : 'planning';
    const response = await fetch('/toggle-mode', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({ mode: newMode }),
    });
    
    if (response.ok) {
      currentMode = newMode;
      updateModeToggle();
    }
  } catch (error) {
    console.error('Failed to toggle mode:', error);
  }
}
let messages = [];
let functionCalls = [];
let textInput = "";
let dataChannel = null;
let pc = null;
let isMuted = false;
let audioTrack = null;
let audioContext = null;
let analyser = null;
let dataArray = null;
let volumeMeterCanvas = document.getElementById("volumeMeter");
let volumeMeterCtx = volumeMeterCanvas.getContext("2d");

// DOM Elements
const connectButton = document.getElementById("connectButton");
const muteButton = document.getElementById("muteButton");
const textInputArea = document.getElementById("textInput");
const sendButton = document.getElementById("sendButton");
const messagesContainer = document.getElementById("messages");
const functionCallsContainer = document.getElementById("functionCalls");
let contexts = [];

async function fetchContexts() {
  try {
    const response = await fetch("/contexts");
    const data = await response.json();
    contexts = data;
  } catch (error) {
    console.error("Failed to fetch contexts:", error);
  }
}

function rawTextToHTML(text) {
  let test = text.replace(/(?:\r\n|\r|\n)/g, "<br>");
  return test;
}

async function updateTranscript() {
  try {
    const transcript = messages
      .map(msg => `${msg.type === 'user' ? 'You' : 'Assistant'}: ${msg.content}`)
      .join('\n\n');
    
    await fetch('/update-transcript', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        content: transcript
      })
    });
  } catch (error) {
    console.error('Failed to update transcript:', error);
  }
}

function updateUI() {
  // Update transcript file whenever UI updates
  updateTranscript();
  // Update connect button
  connectButton.textContent = isConnecting
    ? "Connecting..."
    : isConnected
    ? "Connected"
    : "Start Session";
  connectButton.disabled = isConnecting;

  // Update mute button
  muteButton.style.display = audioTrack ? "block" : "none";
  muteButton.textContent = isMuted ? "Unmute" : "Mute";
  muteButton.disabled = !audioTrack;

  // Update messages
  messagesContainer.innerHTML = messages
    .map(
      (message) => `
        <div class="glass-card" style="margin-bottom: 1rem; ${
          message.type === "user" ? "margin-left: auto;" : "margin-right: auto;"
        } max-width: 80%;">
            ${rawTextToHTML(message.content)}
        </div>
    `
    )
    .join("");

  // Update function calls
  functionCallsContainer.innerHTML = functionCalls
    .map(
      (call) => `
        <div class="glass-card">
            <span class="function-name">${call.name}</span>
            <pre class="function-args">${call.args}</pre>
        </div>
    `
    )
    .join("");

  // Scroll to bottom of function calls and messages
  functionCallsContainer.scrollTop = functionCallsContainer.scrollHeight;
  messagesContainer.scrollTop = messagesContainer.scrollHeight;
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

  messages.push({ type: "user", content: text });
  textInputArea.value = "";
  updateUI();
}

async function requestVoiceCommentary(message) {
  dataChannel.send(
    JSON.stringify({
      type: "conversation.item.create",
      item: {
        type: "message",
        role: "user",
        content: [
          {
            type: "input_text",
            text: message,
          },
        ],
      },
    })
  );
}

async function init() {
  isConnecting = true;
  updateUI();

  try {
    // Get session data from our server
    const response = await fetch("/api/sessions", {
      method: "POST",
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
    dataChannel.addEventListener("open", async () => {
      isConnecting = false;
      isConnected = true;
      updateUI();
      console.log("Connected to OpenAI Realtime API");

      // First fetch contexts before sending function config
      await fetchContexts();

      // Create context enum from fetched contexts
      const contextEnum = contexts.map((ctx) => ctx.filename);

      const functionConfig = {
        type: "session.update",
        session: {
          input_audio_format: "pcm16",
          output_audio_format: "pcm16",
          input_audio_transcription: {
            model: "whisper-1",
          },
          tools: [
            {
              type: "function",
              name: "toggle_microphone",
              description: "Toggle the microphone mute state",
              parameters: {
                type: "object",
                properties: {
                  action: {
                    type: "string",
                    enum: ["mute", "unmute"],
                    description: "Whether to mute or unmute the microphone",
                  },
                },
                required: ["action"],
              },
            },
            {
              type: "function",
              name: "web_search",
              description: "Search the web for information",
              parameters: {
                type: "object",
                properties: {
                  question: {
                    type: "string",
                    description: "The question to ask",
                  },
                },
                required: ["question"],
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

    // Set up audio analysis
    audioContext = new AudioContext();
    const source = audioContext.createMediaStreamSource(ms);
    analyser = audioContext.createAnalyser();
    analyser.fftSize = 256;
    source.connect(analyser);
    dataArray = new Uint8Array(analyser.frequencyBinCount);

    // Start volume meter animation
    function drawVolumeMeter() {
      if (!analyser) return;

      analyser.getByteFrequencyData(dataArray);
      const average = dataArray.reduce((a, b) => a + b) / dataArray.length;
      const volume = average / 256; // Normalize to 0-1

      volumeMeterCtx.clearRect(
        0,
        0,
        volumeMeterCanvas.width,
        volumeMeterCanvas.height
      );
      volumeMeterCtx.fillStyle = isMuted ? "#9ca3af" : "#3b82f6";
      volumeMeterCtx.fillRect(
        0,
        0,
        volumeMeterCanvas.width * volume,
        volumeMeterCanvas.height
      );

      requestAnimationFrame(drawVolumeMeter);
    }
    drawVolumeMeter();

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
        handleFunctionCall(call);
        updateUI();
      }

      // Handle audio transcript events
      if (event.type === "response.output_item.done") {
        // Add completed output item transcript to messages
        if (
          event.item?.content?.[0]?.type === "audio" &&
          event.item.content[0].transcript
        ) {
          messages.push({
            type: "assistant",
            content: event.item.content[0].transcript,
          });
          updateUI();
        }
      } else if (
        event.type === "conversation.item.input_audio_transcription.completed"
      ) {
        if (event.transcript) {
          messages.push({
            type: "user",
            content: event.transcript,
          });
          updateUI();
        }
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
connectButton.addEventListener("click", init);
document.getElementById('modeToggle')?.addEventListener('click', toggleMode);

muteButton.addEventListener("click", () => {
  if (audioTrack) {
    isMuted = !isMuted;
    audioTrack.enabled = !isMuted;
    updateUI();
  }
});

sendButton.addEventListener("click", handleSendMessage);

textInputArea.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    handleSendMessage();
  }
});

async function handleFunctionCall(call) {
  try {
    const args = JSON.parse(call.arguments);
    let response;

    switch (call.name) {
      case "toggle_microphone":
        if (audioTrack) {
          if (args.action === "mute" && !isMuted) {
            isMuted = true;
            audioTrack.enabled = false;
          } else if (args.action === "unmute" && isMuted) {
            isMuted = false;
            audioTrack.enabled = true;
          }
          messages.push({
            type: "assistant",
            content: `Microphone is now ${isMuted ? "muted" : "unmuted"}`,
          });
          updateUI();
          return;
        }
        throw new Error("No microphone available");

      case "web_search":
        const searchResponse = await fetch("/web-search", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            question: args.question,
          }),
        });
        const searchResult = await searchResponse.json();

        messages.push({
          type: "assistant",
          content: `Search: ${args.question} Result: ${searchResult}`,
        });

        break;

      default:
        console.warn("Unknown function call:", call.name);
        return;
    }

    // Update UI with any results from the function calls
    updateUI();
  } catch (error) {
    console.error("Error handling function call:", error);
    messages.push({ type: "assistant", content: `Error: ${error.message}` });
    updateUI();
  }
}

// Initial UI update
updateUI();
