// Global state
let isConnecting = false;
let isConnected = false;
let contexts = [];
let caption = "";
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
let animationId = null;

// DOM Elements
const volumeMeter = document.getElementById("volumeMeter");
const volumeCtx = volumeMeter.getContext("2d");
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
      model: "",  // Will be set by the backend
      voice: "ash",
      instructions:
        "You're name is COLOSSUS. You are a lighthearted, and serious AI that takes code seriously, but you have some wit.  Avoid saying anything that sounds like raw code or json. You are a helpful assistant working with a user to understand and modify a codebase. You can help answer questions about the codebase and make changes to the codebase. You talk very quickly and concisely so I don't have to hear alot of words.  Any time i'm talking about wanting to change something, it's almost always likely a change to the codebase.  Almost any time i'm asking a question, it's usually about the codebase.",
    }),
  });
  return response.json();
}

async function fetchContexts() {
  try {
    const response = await fetch("/contexts");
    contexts = await response.json();
    updateContextsUI();
  } catch (error) {
    console.error("Failed to fetch contexts:", error);
  }
}

function updateContextsUI() {
  const contextsList = document.getElementById("contextsList");
  contextsList.innerHTML = contexts
    .map(
      (context) => `
    <button 
      onclick="handleContextSelect('${context.filename}')"
      class="glass-card" 
      style="width: 100%; text-align: left; margin-bottom: 0.5rem; background: rgba(59, 130, 246, 0.1); color: #3b82f6; font-weight: 500;">
      ${context.filename}
    </button>
  `
    )
    .join("");
}

async function handleContextSelect(filename) {
  try {
    await fetch("/select-context", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
      },
      body: JSON.stringify({ filename }),
    });
  } catch (error) {
    console.error("Failed to select context:", error);
  }
}

async function init() {
  setConnectingState(true);
  await fetchContexts();
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
              name: "change_code",
              description: "Request to change the codebase",
              parameters: {
                type: "object",
                properties: {
                  change: {
                    type: "string",
                    description: "The change to make",
                  },
                },
                required: ["change"],
              },
            },
            {
              type: "function",
              name: "ask_question",
              description: "Ask a question about the codebase",
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
      dc.send(JSON.stringify(functionConfig));
    });

    dc.addEventListener("message", handleMessage);

    const ms = await navigator.mediaDevices.getUserMedia({ audio: true });
    const track = ms.getTracks()[0];
    audioTrack = track;
    newPc.addTrack(track);

    // Set up audio analysis
    audioContext = new AudioContext();
    const source = audioContext.createMediaStreamSource(ms);
    analyser = audioContext.createAnalyser();
    analyser.fftSize = 256;
    source.connect(analyser);

    dataArray = new Uint8Array(analyser.frequencyBinCount);

    // Start volume meter animation
    function drawVolumeMeter() {
      analyser.getByteFrequencyData(dataArray);
      const volume = dataArray.reduce((a, b) => a + b) / dataArray.length;

      volumeCtx.clearRect(0, 0, volumeMeter.width, volumeMeter.height);

      // Draw background
      volumeCtx.fillStyle = "rgba(0, 0, 0, 0.1)";
      volumeCtx.fillRect(0, 0, volumeMeter.width, volumeMeter.height);

      // Draw volume level
      const gradient = volumeCtx.createLinearGradient(
        0,
        0,
        volumeMeter.width,
        0
      );
      gradient.addColorStop(0, "#3b82f6");
      gradient.addColorStop(1, "#2563eb");
      volumeCtx.fillStyle = gradient;

      const width = (volume / 255) * volumeMeter.width;
      volumeCtx.fillRect(0, 0, width, volumeMeter.height);

      animationId = requestAnimationFrame(drawVolumeMeter);
    }

    drawVolumeMeter();

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

    // Handle the function call
    handleFunctionCall(call.name, JSON.parse(call.arguments));
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

async function handleFunctionCall(name, args) {
  let response;
  if (dataChannel) {
    dataChannel.send(
      JSON.stringify({
        type: "conversation.item.create",
        item: {
          type: "message",
          role: "user",
          content: [
            {
              type: "input_text",
              text: `Say that you're acknowleding that you're going to try to perform the action and that it might take a little while: ${JSON.stringify(
                args
              )}`,
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
  }
  try {
    switch (name) {
      case "change_code":
        response = await fetch("/change-code", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ change: args.change }),
        });
        break;
      case "ask_question":
        response = await fetch("/ask-question", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ question: args.question }),
        });
        break;
      default:
        console.warn("Unknown function:", name);
        return;
    }

    const responseText = await response.text();
    messages.push({ type: "assistant", content: responseText });
    updateMessagesUI();

    // For ask_question, send a follow-up request to OpenAI to summarize vocally
    if (dataChannel) {
      dataChannel.send(
        JSON.stringify({
          type: "conversation.item.create",
          item: {
            type: "message",
            role: "user",
            content: [
              {
                type: "input_text",
                text: `I'd like you to summarize this technical response in a more conversational way using your voice in a single sentence.  Phrase your wording as if you are already done.: <AiderLog>${responseText}</AiderLog>`,
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
    }
  } catch (error) {
    console.error("Function call failed:", error);
    messages.push({
      type: "assistant",
      content: "Sorry, there was an error processing your request.",
    });
    updateMessagesUI();
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

function stopVolumeMeter() {
  if (animationId) {
    cancelAnimationFrame(animationId);
    animationId = null;
  }
  if (volumeCtx) {
    volumeCtx.clearRect(0, 0, volumeMeter.width, volumeMeter.height);
    volumeCtx.fillStyle = "rgba(0, 0, 0, 0.1)";
    volumeCtx.fillRect(0, 0, volumeMeter.width, volumeMeter.height);
  }
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
  muteButton.style.display = state ? "block" : "none";
}

// Initial load
fetchContexts();

// Event Listeners
connectButton.addEventListener("click", init);

muteButton.addEventListener("click", () => {
  if (audioTrack) {
    isMuted = !isMuted;
    audioTrack.enabled = !isMuted;
    muteButton.textContent = isMuted ? "Unmute" : "Mute";
    if (isMuted) {
      stopVolumeMeter();
    } else if (analyser) {
      drawVolumeMeter();
    }
  }
});

sendButton.addEventListener("click", handleSendMessage);

textInputElement.addEventListener("keydown", (e) => {
  if (e.key === "Enter" && !e.shiftKey) {
    e.preventDefault();
    handleSendMessage();
  }
});
