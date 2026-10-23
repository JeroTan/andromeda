import { root, useEffect, useState, useGlobalProps } from "@lynx-js/react";
import "./style.css";

type Status = {
  enabled: boolean;
  service_ready: boolean;
  paired: boolean;
  session_active: boolean;
  session_expired: boolean;
  last_contact_seconds: number;
  pairing_seconds: number;
  session_seconds: number;
  timer_minutes: number;
  dark: boolean;
  title_size: number;
  body_size: number;
  small_size: number;
  button_size: number;
  developer: boolean;
  error?: string;
};
type Action =
  | "refresh"
  | "accessibility-settings"
  | "settings"
  | "session-settings"
  | "back"
  | "timer-off"
  | "timer-15"
  | "timer-30"
  | "timer-60"
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
        text-single-line-vertical-align="center"
        accessibility-element={false}
      >
        {children}
      </text>
    </view>
  );
}

function App() {
  const { screen = "home", refresh_id = 0 } = useGlobalProps() as {
    screen?: string;
    refresh_id?: number;
  };
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
  }, [refresh_id]);
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
  const badgeSize = Math.max(36, Math.round((status.small_size || 12) * 2));
  const step = !status.enabled
    ? 0
    : !status.paired
      ? 1
      : status.session_active
        ? 3
        : 2;
  const pairing = status.pairing_seconds > 0;
  const waitingForService = status.enabled && !status.service_ready;
  const title = waitingForService
    ? "Permission set.\nConnecting…"
    : [
        "Let’s make\ncontact.",
        "Your phone.\nIn good company.",
        "All set.\nYou lead.",
        "A little\nmore reach.",
      ][step];
  const description = waitingForService
    ? "Accessibility permission is already enabled. Waiting for Android to connect the service; this updates automatically. Your saved pairing is kept."
    : [
        "Enable accessibility so your paired computer can read accessible screens, enter text and perform gestures during a session.",
        "Your phone and computer need an introduction. Open pairing here, then choose this phone in Andromeda on your computer.",
        "Start a session when you’re ready to use accessibility tools. You decide when it begins and ends.",
        "This phone is ready to accept requests from your paired computer. Keep your task in view, and stop whenever you need.",
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
        <view className="topbar-actions">
          <view
            flatten={false}
            className="settings-small"
            accessibility-element={true}
            accessibility-traits="button"
            accessibility-label="Settings"
            accessibility-enable-tap={true}
            bindtap={() => act("settings")}
          >
            <text
              flatten={false}
              className="small"
              style={type(status.small_size)}
              accessibility-element={false}
            >
              Settings
            </text>
          </view>
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
              text-single-line-vertical-align="center"
              accessibility-element={false}
            >
              Stop
            </text>
          </view>
        </view>
      </view>
      <scroll-view className="scroll" scroll-y={true}>
        <view className="content">
          {screen !== "home" ? (
            <view className="panel controls">
              <ActionButton
                status={status}
                busy={busy}
                onAction={act}
                action="back"
                secondary={true}
              >
                {screen === "session" ? "Back to Settings" : "Back to home"}
              </ActionButton>
              <text
                flatten={false}
                className="status-title"
                style={type(status.body_size)}
                accessibility-element={true}
                accessibility-heading={true}
              >
                {screen === "session" ? "Session timer" : "Settings"}
              </text>
              {screen === "settings" ? (
                <>
                  <text
                    flatten={false}
                    className="small"
                    style={type(status.small_size)}
                    accessibility-element={true}
                  >
                    Choose a section to change its settings.
                  </text>
                  <ActionButton
                    status={status}
                    busy={busy}
                    onAction={act}
                    action="session-settings"
                    secondary={true}
                  >
                    Session
                  </ActionButton>
                  <text
                    flatten={false}
                    className="small"
                    style={type(status.small_size)}
                    accessibility-element={true}
                  >{`Session timer · ${status.timer_minutes === 0 ? "Off" : status.timer_minutes === 60 ? "1 hour" : `${status.timer_minutes} minutes`}`}</text>
                </>
              ) : (
                <>
                  <text
                    flatten={false}
                    className="body"
                    style={type(status.body_size)}
                    accessibility-element={true}
                  >
                    Off keeps your session open until you stop it. An optional
                    timer stops it automatically. Your choice is saved and
                    applies when you next start a session.
                  </text>
                  {(
                    [
                      [0, "Off", "timer-off"],
                      [15, "15 minutes", "timer-15"],
                      [30, "30 minutes", "timer-30"],
                      [60, "1 hour", "timer-60"],
                    ] as const
                  ).map(([minutes, label, action]) => (
                    <ActionButton
                      key={action}
                      status={status}
                      busy={busy}
                      onAction={act}
                      action={action}
                      secondary={status.timer_minutes !== minutes}
                    >{`${label}${status.timer_minutes === minutes ? ", selected" : ""}`}</ActionButton>
                  ))}
                </>
              )}
              {error ? (
                <text
                  flatten={false}
                  className="error"
                  style={type(status.body_size)}
                  accessibility-element={true}
                >
                  {error}
                </text>
              ) : null}
            </view>
          ) : (
            <>
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
                {waitingForService ? (
                  <ActionButton
                    status={status}
                    busy={busy}
                    onAction={act}
                    action="refresh"
                  >
                    Check connection
                  </ActionButton>
                ) : step === 0 ? (
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
                    Start session
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
                    : waitingForService
                      ? "Accessibility enabled · service connecting"
                      : pairing
                        ? "Waiting for your computer"
                        : status.session_active
                          ? seconds < 0
                            ? "Session active · until you stop it"
                            : `Session active · ${remaining} left`
                          : status.session_expired
                            ? "Session expired · start a new session to continue"
                            : status.paired
                              ? "Computer paired · session stopped"
                              : "Ready to pair"}
                </text>
                {status.paired && (
                  <text
                    flatten={false}
                    accessibility-element={true}
                    className="small"
                    style={type(status.small_size)}
                  >
                    {status.last_contact_seconds < 0
                      ? "Waiting for a computer request. Pairing does not mean a computer is connected."
                      : `Last computer request ${status.last_contact_seconds}s ago. Connection is checked when a request arrives.`}
                  </text>
                )}
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
                    <view
                      className={`step-number ${step > index ? "complete" : ""}`}
                      style={{
                        width: `${badgeSize}px`,
                        height: `${badgeSize}px`,
                        borderRadius: `${badgeSize / 2}px`,
                      }}
                    >
                      <text
                        accessibility-element={false}
                        flatten={false}
                        className="step-number-text"
                        style={type(status.small_size)}
                        text-single-line-vertical-align="center"
                      >{`0${index + 1}`}</text>
                    </view>
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
                  details
                    ? "Hide connection controls"
                    : "More connection controls"
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
                    This bridge stays inside the USB/ADB connection.
                    Accessibility permission belongs to you and can be disabled
                    at any time.
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
                  {status.service_ready && (
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
                  {status.enabled && !status.service_ready && (
                    <ActionButton
                      status={status}
                      busy={busy}
                      onAction={act}
                      action="accessibility-settings"
                      secondary={true}
                    >
                      Open accessibility settings
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
            </>
          )}
        </view>
      </scroll-view>
    </view>
  );
}
root.render(<App />);
