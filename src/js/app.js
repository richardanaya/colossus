"use client";

import { createSession } from "./actions/session";
import { useState } from "react";

export default function Home() {
  const [isConnecting, setIsConnecting] = useState(false);
  const [isConnected, setIsConnected] = useState(false);
  const [caption, setCaption] = useState("");
  const [messages, setMessages] = useState<Array<{ type: 'assistant' | 'user', content: string }>>([]);
  const [functionCalls, setFunctionCalls] = useState<
    Array<{ name: string; args: string }>
  >([]);
  const [textInput, setTextInput] = useState("");
  const [dataChannel, setDataChannel] = useState<RTCDataChannel | null>(null);

  const handleSendMessage = async () => {
    if (!textInput.trim()) return;

    if (dataChannel) {
      console.log("Sending message to OpenAI:", textInput);
      dataChannel.send(
        JSON.stringify({
          type: "conversation.item.create",
          item: {
            type: "message",
            role: "user",
            content: [
              {
                type: "input_text",
                text: textInput,
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
      setMessages(prev => [...prev, { type: 'user', content: textInput }]);
      setTextInput("");
    }
  };
  const [pc, setPc] = useState<RTCPeerConnection | null>(null);
  const [isMuted, setIsMuted] = useState(false);
  const [audioTrack, setAudioTrack] = useState<MediaStreamTrack | null>(null);

  async function init() {
    setIsConnecting(true);
    try {
      // Get session data from our server action
      const data = await createSession();
      const EPHEMERAL_KEY = data.client_secret.value;

      // Create a peer connection
      const newPc = new RTCPeerConnection();
      setPc(newPc);

      // Set up to play remote audio from the model
      const audioEl = document.createElement("audio");
      audioEl.autoplay = true;
      newPc.ontrack = (e) => (audioEl.srcObject = e.streams[0]);

      // Configure initial session with onyx voice and set up data channel
      const dc = newPc.createDataChannel("oai-events");
      setDataChannel(dc);
      dc.addEventListener("open", () => {
        setIsConnecting(false); // Connection is no longer connecting
        setIsConnected(true); // Connection is now established
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
        dc.send(JSON.stringify(functionConfig));
      });

      // Add local audio track for microphone input in the browser
      const ms = await navigator.mediaDevices.getUserMedia({
        audio: true,
      });
      const track = ms.getTracks()[0];
      setAudioTrack(track);
      newPc.addTrack(track);
      dc.addEventListener("message", (e) => {
        const event = JSON.parse(e.data);

        // Log all incoming messages
        console.log("Received from OpenAI:", event);

        // Log partial function call arguments
        if (event.type === "response.function_call_arguments.delta") {
          console.log("Partial function call:", event.delta);
        } else if (
          event.type === "response.done" &&
          event.response.output?.[0]?.type === "function_call"
        ) {
          const call = event.response.output[0];
          setFunctionCalls((prev) => [
            ...prev,
            {
              name: call.name,
              args: call.arguments,
            },
          ]);
        }

        // Handle audio transcript events
        if (event.type === "response.audio_transcript.delta") {
          // Update caption and add to messages
          setCaption(event.delta);
          setMessages(prev => {
            const lastMessage = prev[prev.length - 1];
            if (lastMessage?.type === 'assistant') {
              return [...prev.slice(0, -1), { type: 'assistant', content: lastMessage.content + event.delta }];
            } else {
              return [...prev, { type: 'assistant', content: event.delta }];
            }
          });
        } else if (event.type === "response.done") {
          setCaption("");
        }
      });

      // Start the session using the Session Description Protocol (SDP)
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
      setIsConnecting(false);
    }
  }

  return (
    <main className="flex min-h-screen flex-col items-center justify-between p-8 relative">
      <div className="w-full max-w-7xl mx-auto flex flex-col gap-8">
        {/* Header */}
        <header className="w-full flex justify-between items-center glass-card rounded-xl p-6">
          <h1 className="text-2xl font-bold">AI Assistant</h1>
          <div className="flex gap-4">
            <button
              onClick={init}
              disabled={isConnecting}
              className="px-6 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:bg-gray-400 transition-colors duration-200 font-medium"
            >
              {isConnecting
                ? "Connecting..."
                : isConnected
                ? "Connected"
                : "Start Session"}
            </button>
            <button
              onClick={() => {
                if (audioTrack) {
                  const newMutedState = !isMuted;
                  audioTrack.enabled = !newMutedState;
                  setIsMuted(newMutedState);
                }
              }}
              disabled={!audioTrack}
              className="px-6 py-2 bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:bg-gray-400 transition-colors duration-200 font-medium"
            >
              {isMuted ? "Unmute" : "Mute"}
            </button>
          </div>
        </header>

        {/* Main Content Area */}
        <div className="flex gap-8 h-[calc(100vh-16rem)]">
          {/* Function Calls Panel */}
          <div className="w-1/3 overlay-panel rounded-xl p-6 overflow-hidden flex flex-col">
            <h3 className="text-xl font-bold mb-4">Function Calls</h3>
            <div className="overflow-y-auto flex-1">
              {functionCalls.map((call, i) => (
                <div key={i} className="mb-4 glass-card rounded-lg p-4">
                  <span className="font-mono text-blue-600 dark:text-blue-400">
                    {call.name}
                  </span>
                  <pre className="text-sm text-gray-600 dark:text-gray-300 mt-2 whitespace-pre-wrap">
                    {call.args}
                  </pre>
                </div>
              ))}
            </div>
          </div>

          {/* Text Input Panel */}
          <div className="w-2/3 overlay-panel rounded-xl p-6 flex flex-col">
            <h3 className="text-xl font-bold mb-4">Chat</h3>
            <div className="flex-1 overflow-y-auto mb-4">
              {messages.map((message, index) => (
                <div key={index} className={`glass-card rounded-lg p-4 mb-4 ${
                  message.type === 'user' ? 'ml-auto max-w-[80%]' : 'mr-auto max-w-[80%]'
                }`}>
                  <p className="text-lg">{message.content}</p>
                </div>
              ))}
              {caption && (
                <div className="glass-card rounded-lg p-4 mb-4 mr-auto max-w-[80%] opacity-50">
                  <p className="text-lg">{caption}</p>
                </div>
              )}
            </div>
            <div className="flex flex-col gap-4">
              <div className="flex gap-4">
                <textarea
                  value={textInput}
                  onChange={(e) => setTextInput(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter" && !e.shiftKey) {
                      e.preventDefault();
                      handleSendMessage();
                    }
                  }}
                  className="flex-1 p-4 rounded-lg bg-black/5 dark:bg-white/10 text-foreground placeholder-gray-500 resize-none"
                  placeholder="Type your message here... (Press Enter to send)"
                  rows={3}
                />
                <button
                  onClick={handleSendMessage}
                  className="px-6 py-2 h-fit bg-blue-500 text-white rounded-lg hover:bg-blue-600 transition-colors duration-200 font-medium"
                >
                  Send
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    </main>
  );
}
