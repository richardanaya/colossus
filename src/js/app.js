let isConnecting = false;
let isConnected = false;
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

function updateUI() {
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
  dataChannel.send(
    JSON.stringify({
      type: "response.create",
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
      requestVoiceCommentary(
        "Introduce yourself as Colossus and ask the user how they can help them with their codebase."
      );
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
          tools: [
            {
              type: "function",
              name: "modify_code",
              description: "Request to create or modify code in the codebase",
              parameters: {
                type: "object",
                properties: {
                  action: {
                    type: "string",
                    enum: ["create", "modify"],
                    description: "Whether to create new code or modify existing code",
                  },
                  change: {
                    type: "string",
                    description: "The code change to make or new code to create",
                  },
                  context: {
                    type: "string",
                    enum: contextEnum,
                    description:
                      "The file to create or modify, choose one based on " +
                      JSON.stringify(contextEnum),
                  },
                },
                required: ["action", "change", "context"],
              },
            },
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
              name: "ask_question",
              description: "Ask a question about the codebase",
              parameters: {
                type: "object",
                properties: {
                  question: {
                    type: "string",
                    description: "The question to search for",
                  },
                },
                required: ["question"],
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
                required: ["question", "context"],
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
      case "modify_code":
        requestVoiceCommentary(
          `Could you vocally say that you'll ${args.action === "create" ? "create new code" : "make the changes"} and it might take some time in some appropriate manner to your personality and the conversation.`
        );
        response = await fetch("/change-code", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            action: args.action,
            change: args.change,
            context: args.context,
          }),
        });
        requestVoiceCommentary(
          "Summarize the information retrieved from the operation, try to be breif as this will be spoken (like 2 sentences max). " +
            JSON.stringify(await response.json())
        );
        break;

      case "toggle_microphone":
        if (audioTrack) {
          if (args.action === "mute" && !isMuted) {
            isMuted = true;
            audioTrack.enabled = false;
          } else if (args.action === "unmute" && isMuted) {
            isMuted = false;
            audioTrack.enabled = true;
          }
          requestVoiceCommentary(
            "Could you vocally say you muted the mic with some appropriate confirmation."
          );
          updateUI();
          return;
        }
        throw new Error("No microphone available");

      case "ask_question":
        requestVoiceCommentary(
          "Could you vocally say that you'll look up the question it might take some time in some appropriate manner to your personality and the converesation."
        );
        response = await fetch("/ask-question", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            question: args.question,
            context: args.context,
          }),
        });
        requestVoiceCommentary(
          "Summarize the information retrieved from the operation, try to be breif as this will be spoken (like 2 sentences max). " +
            JSON.stringify(await response.json())
        );
        break;

      case "web_search":
        requestVoiceCommentary(
          "I'll search the web for that information. Give me a moment."
        );
        response = await fetch("/web-search", {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            question: args.question,
          }),
        });
        const searchResult = await response.json();
        requestVoiceCommentary(
          "Here's what I found from searching the web: " + searchResult
        );
        break;

      default:
        console.warn("Unknown function call:", call.name);
        return;
    }

    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const responseData = await response.json();
    messages.push({
      type: "assistant",
      content: JSON.stringify(responseData),
    });
    updateUI();
  } catch (error) {
    console.error("Error handling function call:", error);
    messages.push({ type: "assistant", content: `Error: ${error.message}` });
    updateUI();
  }
}

// Initial UI update
updateUI();
