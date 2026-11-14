import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

import "./App.css";

function App() {
  const [message, setMessage] = useState("");
  const [screenCaptured, setScreenCaptured] = useState(false);
  const [screenshot, setScreenshot] = useState<string | null>(null);
  const [response, setResponse] = useState("");
  const [isThinking, setIsThinking] = useState(false);
  const [error, setError] = useState("");

  const inputRef = useRef<HTMLTextAreaElement>(null);

  const [activeWindow, setActiveWindow] = useState<{
    title: string;
    process_id: number;
  } | null>(null);

  const refreshActiveWindow = async () => {
   try {
     const context = await invoke<{
       title: string;
       process_id: number;
      }>("get_active_window_context");

      setActiveWindow(context);
    } catch (error) {
      console.error("Failed to detect active window:", error);
    }
  };

  useEffect(() => {
    inputRef.current?.focus();
    refreshActiveWindow();
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      unlisten = await listen(
        "context-active-window-ready",
        () => {
          refreshActiveWindow();
        }
      );
    };

    setupListener();

    return () => {
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let mounted = true;

    const setupListener = async () => {
      unlisten = await listen(
        "context-screenshot-ready",
        async () => {
          console.log("FRONTEND: SCREENSHOT RECEIVED");

          if (!mounted) return;

          try {
            const bytes = await invoke<number[] | null>(
              "get_screenshot"
            );

            if (!bytes || bytes.length === 0) {
              console.log(
                "FRONTEND: NO SCREENSHOT DATA"
              );
              return;
            }

            console.log(
              "FRONTEND: SCREENSHOT BYTES:",
              bytes.length
            );

            const uint8Array = new Uint8Array(bytes);

            let binary = "";

            const chunkSize = 8192;

            for (
              let i = 0;
              i < uint8Array.length;
              i += chunkSize
            ) {
              const chunk = uint8Array.subarray(
                i,
                Math.min(
                  i + chunkSize,
                  uint8Array.length
                )
              );

              binary += String.fromCharCode(...chunk);
            }

            const base64 = btoa(binary);

            if (mounted) {
              setScreenshot(
                `data:image/png;base64,${base64}`
              );

              setScreenCaptured(true);
            }
          } catch (error) {
            console.error(
              "FRONTEND: FAILED TO GET SCREENSHOT:",
              error
            );
          }
        }
      );
    };

    setupListener();

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, []);

const submitMessage = async () => {
  const trimmed = message.trim();

  if (!trimmed || isThinking) return;

  console.log("User message:", trimmed);

  setMessage("");
  setResponse("");
  setError("");
  setIsThinking(true);

  try {
    const result = await invoke<string>("ask_ai", {
      question: trimmed,
    });

    console.log("AI RESPONSE:", result);

    setResponse(result);
  } catch (error) {
    console.error("AI REQUEST FAILED:", error);

    setError(
      error instanceof Error
        ? error.message
        : String(error)
    );
  } finally {
    setIsThinking(false);
  }
};

  return (
    <main className="app">
      <section className="assistant">
        <header className="header">
          <div className="brand">
            <div className="logo">C</div>

            <div>
              <div className="name">Context</div>

              <div className="status">
                Ready
              </div>
            </div>
          </div>

          <button
            className="close-button"
            onClick={async () => {
              await getCurrentWindow().hide();
            }}
            aria-label="Close"
          >
            ×
          </button>
        </header>

        <div className="conversation">
          {isThinking ? (
            <div className="empty-state">
              <div className="empty-title">
                Thinking...
              </div>

              <div className="empty-subtitle">
                Analyzing your screen.
              </div>
            </div>
          ) : error ? (
            <div className="empty-state">
              <div className="empty-title">
                Something went wrong
              </div>

              <div className="empty-subtitle">
                {error}
              </div>
            </div>
          ) : response ? (
            <div
              style={{
                width: "100%",
                height: "100%",
                overflowY: "auto",
                padding: "8px",
                boxSizing: "border-box",
              }}
            >
              <div
                style={{
                  whiteSpace: "pre-wrap",
                  fontSize: "14px",
                  lineHeight: "1.6",
                }}
              >
                {response}
              </div>
            </div>
          ) : screenshot ? (
            <div
              style={{
                width: "100%",
                height: "100%",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                padding: "20px",
                boxSizing: "border-box",
              }}
            >
              <img
                src={screenshot}
                alt="Captured screen"
                style={{
                  maxWidth: "100%",
                  maxHeight: "100%",
                  objectFit: "contain",
                  borderRadius: "12px",
                }}
              />
            </div>
          ) : (
            <div className="empty-state">
              <div className="empty-title">
                What can I help with?
              </div>

              <div className="empty-subtitle">
                Ask about what's on your screen.
              </div>
            </div>
          )}
        </div>

        <div className="composer">
          <textarea
            ref={inputRef}
            value={message}
            onChange={(event) => {
              setMessage(event.target.value);
            }}
            onKeyDown={(event) => {
              if (
                event.key === "Enter" &&
                !event.shiftKey
              ) {
                event.preventDefault();
                submitMessage();
              }
            }}
            placeholder="Ask anything..."
            rows={1}
          />

          <button
            className="send-button"
            onClick={submitMessage}
            disabled={!message.trim()}
            aria-label="Send"
          >
            ↑
          </button>
        </div>

        <footer className="footer">
          <span title={activeWindow?.title ?? "Unknown window"}>
           {activeWindow?.title || "Detecting window..."}
          </span>
          <span>Screen context</span>

          <span className="context-status">
            ●{" "}
            {screenCaptured
              ? "Captured"
              : "Waiting"}
          </span>
        </footer>
      </section>
    </main>
  );
}

export default App;