import { root, useEffect, useState } from "@lynx-js/react";
import "./style.css";

type Status = {
  enabled: boolean;
  paired: boolean;
  session_active: boolean;
  pairing_seconds: number;
  session_seconds: number;
  dark: boolean;
  title_size: number;
  body_size: number;
  small_size: number;
  button_size: number;
  developer: boolean;
  error?: string;
};
type Action =
  | "accessibility"
  | "pair"
  | "start"
  | "stop"
  | "forget"
  | "disable"
  | "diagnostics";
interface CompanionBridge {
  status(done: (value: Status) => void): void;
  action(name: Action, done: (value: Status) => void): void;
}
function bridge(): CompanionBridge {
  return NativeModules.Companion;
}

function ActionButton({
  children,
  action,
  secondary = false,
  disabled = false,
  status,
  busy,
  onAction,
}: {
  children: string;
  action: Action;
  secondary?: boolean;
  disabled?: boolean;
  status: Status;
  busy: boolean;
  onAction: (action: Action) => void;
}) {
  const unavailable = disabled || (busy && action !== "stop");
  return (
    <view
      flatten={false}
      className={`button ${secondary ? "secondary" : "primary"} ${unavailable ? "disabled" : ""}`}
      accessibility-element={true}
      accessibility-traits="button"
      accessibility-label={unavailable ? `${children}, unavailable` : children}
      accessibility-enable-tap={!unavailable}
      bindtap={() => {
        if (!unavailable) onAction(action);
      }}
    >
      <text
        flatten={false}
        className="button-text"
        style={{ fontSize: `${status.button_size}px` }}
        accessibility-element={false}
      >
        {children}
      </text>
    </view>
  );
}

function App() {
  const [status, setStatus] = useState<Status | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [details, setDetails] = useState(false);
  useEffect(() => {
    let mounted = true;
    const refresh = () =>
      bridge().status((value) => {
        if (mounted) setStatus(value);
      });
    refresh();
    const timer = setInterval(refresh, 1000);
    return () => {
      mounted = false;
      clearInterval(timer);
    };
  }, []);
  function act(action: Action) {
    if (busy && action !== "stop") return;
    setBusy(true);
    setError("");
    bridge().action(action, (value) => {
      setStatus(value);
      setError(value.error || "");
      setBusy(false);
    });
  }
  const type = (size: number | undefined) => ({ fontSize: `${size || 16}px` });
  if (!status)
    return (
      <view className="page">
        <text flatten={false} className="loading" accessibility-element={true}>
          Opening Andromeda…
        </text>
      </view>
    );
  const step = !status.enabled
    ? 0
    : !status.paired
      ? 1
      : status.session_active
        ? 3
        : 2;
  const pairing = status.pairing_seconds > 0;
  const title = [
    "Let’s make\ncontact.",
    "Your phone.\nIn good company.",
    "All set.\nYou lead.",
    "A little\nmore reach.",
  ][step];
  const description = [
    "Enable accessibility so your paired computer can read accessible screens, enter text and perform gestures during a session.",
    "Your phone and computer need an introduction. Open pairing here, then choose this phone in Andromeda on your computer.",
    "Start a session when you’re ready to use accessibility tools. You decide when it begins and ends.",
    "Your computer is connected to this phone’s accessibility tools. Keep your task in view, and stop whenever you need.",
  ][step];
  const seconds = status.session_seconds;
  const remaining = `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
  return (
    <view className={`page ${status.dark ? "dark" : "light"}`}>
      <view className="topbar">
        <text
          flatten={false}
          className="wordmark"
          style={type(status.button_size)}
          accessibility-element={true}
        >
          ANDROMEDA
        </text>
        <view
          flatten={false}
          className="stop-small"
          accessibility-element={true}
          accessibility-traits="button"
          accessibility-label="Stop companion session"
          accessibility-enable-tap={true}
          bindtap={() => act("stop")}
        >
          <text
            flatten={false}
            className="stop-text"
            style={type(status.small_size)}
            accessibility-element={false}
          >
            Stop
          </text>
        </view>
      </view>
      <scroll-view className="scroll" scroll-y={true}>
        <view className="content">
          <view className="hero panel">
            <text
              accessibility-element={true}
              flatten={false}
              className="eyebrow"
              style={type(status.small_size)}
            >
              YOUR PHONE, WITH REACH.
            </text>
            <text
              flatten={false}
              className="headline"
              style={type(status.title_size)}
              accessibility-element={true}
              accessibility-traits="header"
              accessibility-heading={true}
            >
              {title}
            </text>
            <view
              className="art"
              accessibility-element={false}
              important-for-accessibility="no-hide-descendants"
            >
              <view className="ring ring-a" />
              <view className="ring ring-b" />
              <view className="planet" />
              <view className="moon" />
              <view className="star-vertical" />
              <view className="star-horizontal" />
            </view>
            <text
              flatten={false}
              className="body"
              style={type(status.body_size)}
              accessibility-element={true}
            >
              {description}
            </text>
            {step === 0 ? (
              <ActionButton
                status={status}
                busy={busy}
                onAction={act}
                action="accessibility"
              >
                Enable accessibility
              </ActionButton>
            ) : pairing ? (
              <>
                <text
                  flatten={false}
                  className="pairing-time"
                  style={type(status.body_size)}
                  accessibility-element={true}
                >{`Pairing is open · ${status.pairing_seconds}s`}</text>
                <ActionButton
                  status={status}
                  busy={busy}
                  onAction={act}
                  action="stop"
                  secondary={true}
                >
                  Cancel pairing
                </ActionButton>
              </>
            ) : step === 1 ? (
              <ActionButton
                status={status}
                busy={busy}
                onAction={act}
                action="pair"
              >
                Pair this computer
              </ActionButton>
            ) : step === 2 ? (
              <ActionButton
                status={status}
                busy={busy}
                onAction={act}
                action="start"
              >
                Start a 15-minute session
              </ActionButton>
            ) : (
              <ActionButton
                status={status}
                busy={busy}
                onAction={act}
                action="stop"
              >
                Stop session
              </ActionButton>
            )}
            {error ? (
              <text
                flatten={false}
                className="error"
                style={type(status.body_size)}
                accessibility-element={true}
                accessibility-label={error}
              >
                {error}
              </text>
            ) : null}
          </view>
          <view className="status-panel panel">
            <view className="status-heading">
              <text
                accessibility-element={true}
                flatten={false}
                className="eyebrow"
                style={type(status.small_size)}
              >
                CONNECTION / STATUS
              </text>
              <view
                className={`status-dot ${status.session_active ? "active" : ""}`}
              />
            </view>
            <text
              flatten={false}
              className="status-title"
              style={type(status.body_size)}
              accessibility-element={true}
            >
              {!status.enabled
                ? "Accessibility is off"
                : pairing
                  ? "Waiting for your computer"
                  : status.session_active
                    ? `Session active · ${remaining} left`
                    : status.paired
                      ? "Computer paired · session stopped"
                      : "Ready to pair"}
            </text>
            <text
              accessibility-element={true}
              flatten={false}
              className="small"
              style={type(status.small_size)}
            >
              Set up your project and agent connection in the desktop app.
              Desktop Stop pauses all managed work.
            </text>
          </view>
          <view className="steps panel">
            {[
              "Enable accessibility",
              "Pair your computer",
              "Start a session",
            ].map((name, index) => (
              <view className="step" key={name}>
                <text
                  accessibility-element={true}
                  flatten={false}
                  className={`step-number ${step > index ? "complete" : ""}`}
                  style={type(status.small_size)}
                >{`0${index + 1}`}</text>
                <text
                  flatten={false}
                  className="step-label"
                  style={type(status.body_size)}
                  accessibility-element={true}
                >{`${name}${step > index ? " · done" : ""}`}</text>
              </view>
            ))}
          </view>
          <view
            flatten={false}
            className="details-toggle"
            accessibility-element={true}
            accessibility-traits="button"
            accessibility-label={
              details ? "Hide connection controls" : "More connection controls"
            }
            accessibility-enable-tap={true}
            bindtap={() => setDetails(!details)}
          >
            <text
              accessibility-element={true}
              flatten={false}
              className="small"
              style={type(status.small_size)}
            >
              {details ? "Less detail −" : "Connection controls +"}
            </text>
          </view>
          {details && (
            <view className="panel controls">
              <text
                accessibility-element={true}
                flatten={false}
                className="small"
                style={type(status.small_size)}
              >
                This bridge stays inside the USB/ADB connection. Accessibility
                permission belongs to you and can be disabled at any time.
              </text>
              {status.paired && (
                <ActionButton
                  status={status}
                  busy={busy}
                  onAction={act}
                  action="forget"
                  secondary={true}
                >
                  Forget this computer
                </ActionButton>
              )}
              {status.enabled && (
                <ActionButton
                  status={status}
                  busy={busy}
                  onAction={act}
                  action="disable"
                  secondary={true}
                >
                  Disable accessibility
                </ActionButton>
              )}
              {status.developer && (
                <ActionButton
                  status={status}
                  busy={busy}
                  onAction={act}
                  action="diagnostics"
                  secondary={true}
                >
                  Open gesture test pad
                </ActionButton>
              )}
            </view>
          )}
          <text
            accessibility-element={true}
            flatten={false}
            className="footer"
            style={type(status.small_size)}
          >
            {"A LITTLE MORE REACH.\nANDROMEDA / 0.1.0"}
          </text>
        </view>
      </scroll-view>
    </view>
  );
}
root.render(<App />);
