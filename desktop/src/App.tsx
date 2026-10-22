import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Project = {
  id: string;
  name: string;
  root: string;
  serial: string | null;
  allowed_tools: string[];
  instruction_file: string;
  enabled: boolean;
};
type Tool = { name: string; description: string; read_only: boolean };
type Approval = {
  id: number;
  project_name: string;
  project_root: string;
  serial: string | null;
  tool: string;
  arguments: unknown;
  expires_at_ms: number;
};
type Snapshot = {
  projects: Project[];
  paused: boolean;
  pause_generation: number;
  approvals: Approval[];
  connections: {
    project_id: string;
    clients: number;
    initialization_error: string | null;
  }[];
  tools: Tool[];
};
type Device = { serial: string; state: string; details: string };
type Change = {
  id: number;
  path: string;
  replaces_existing: boolean;
  before: string;
  after: string;
};
type Instructions = { path: string; content: string | null; starter: string };
type Preview = { data_url: string; elapsed_ms: number; serial: string };
type DialogState =
  | { kind: "connection"; client: string; change: Change }
  | { kind: "instructions"; project: Project; document: Instructions }
  | { kind: "approval"; approval: Approval };

const label = (tool: string) => tool.replace("adm_", "").replace(/_/g, " ");
const message = (error: unknown) =>
  typeof error === "object" && error !== null && "message" in error
    ? String(error.message)
    : String(error);

function Icon({
  name,
  size = 20,
}: {
  name:
    "arrow" | "plus" | "moon" | "sun" | "stop" | "phone" | "folder" | "close";
  size?: number;
}) {
  const paths = {
    arrow: "M5 12h14M13 6l6 6-6 6",
    plus: "M12 5v14M5 12h14",
    moon: "M20 14A8 8 0 0 1 10 4a8 8 0 1 0 10 10Z",
    sun: "M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6 1.4 1.4m10 10 1.4 1.4M5.6 18.4 7 17m10-10 1.4-1.4M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0",
    stop: "M6 6h12v12H6Z",
    phone: "M7 3h10v18H7ZM10 17h4",
    folder: "M3 7V5h7l2 2h9v13H3Z",
    close: "m6 6 12 12M6 18 18 6",
  };
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.7"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={paths[name]} />
    </svg>
  );
}

function Star({ size = 34 }: { size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 48 48"
      fill="none"
      stroke="currentColor"
      strokeWidth="4"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <path d="M24 5v38M5 24h38M11 11l26 26M11 37l26-26" />
    </svg>
  );
}

function Orbit({ small = false }: { small?: boolean }) {
  return (
    <div className={`orbit-art ${small ? "small" : ""}`} aria-hidden="true">
      <div className="orbit-ring ring-one" />
      <div className="orbit-ring ring-two" />
      <div className="orbit-ring ring-three" />
      <div className="planet" />
      <span className="star star-one">
        <Star size={56} />
      </span>
      <span className="star star-two">+</span>
      <span className="satellite" />
    </div>
  );
}

function Modal({
  title,
  onClose,
  onStop,
  children,
}: {
  title: string;
  onClose: () => void;
  onStop: () => void;
  children: ReactNode;
}) {
  const ref = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const dialog = ref.current!;
    const previous = document.activeElement;
    dialog.showModal();
    dialog.querySelector("h2")?.focus();
    return () => {
      dialog.close();
      if (previous instanceof HTMLElement && previous.isConnected)
        previous.focus();
    };
  }, []);
  return (
    <dialog
      ref={ref}
      className="modal"
      onCancel={(event) => {
        event.preventDefault();
        onClose();
      }}
      aria-labelledby="modal-title"
    >
      <header className="modal-header">
        <h2 id="modal-title" tabIndex={-1}>
          {title}
        </h2>
        <button className="stop-button" onClick={onStop}>
          <Icon name="stop" />
          Stop
        </button>
        <button
          className="icon-button"
          onClick={onClose}
          aria-label="Close dialog"
        >
          <Icon name="close" />
        </button>
      </header>
      {children}
    </dialog>
  );
}

function App() {
  const [data, setData] = useState<Snapshot | null>(null);
  const [selected, setSelected] = useState<string>("");
  const [draft, setDraft] = useState<Project | null>(null);
  const [deviceList, setDeviceList] = useState<Device[]>([]);
  const [deviceError, setDeviceError] = useState("");
  const [busy, setBusy] = useState("");
  const [notice, setNotice] = useState("");
  const [error, setError] = useState("");
  const [dialog, setDialog] = useState<DialogState | null>(null);
  const [documentText, setDocumentText] = useState("");
  const [replace, setReplace] = useState(false);
  const [preview, setPreview] = useState<Preview | null>(null);
  const previewGeneration = useRef(0);
  const [theme, setTheme] = useState(
    () => localStorage.getItem("andromeda-theme") || "light",
  );
  const [now, setNow] = useState(Date.now());
  const selectedRef = useRef(selected);
  selectedRef.current = selected;
  const project = data?.projects.find((p) => p.id === selected);
  const connection = data?.connections.find(
    (status) => status.project_id === selected,
  );
  const dirty = !!(
    draft &&
    project &&
    JSON.stringify(draft) !== JSON.stringify(project)
  );
  const paused = data?.paused ?? true;

  const refresh = useCallback(async () => {
    const state = await invoke<Snapshot>("snapshot");
    setData(state);
    setSelected((current) => current || state.projects[0]?.id || "");
  }, []);
  const refreshDevices = useCallback(async () => {
    try {
      setDeviceList(await invoke<Device[]>("devices"));
      setDeviceError("");
    } catch (error) {
      setDeviceError(message(error));
    }
  }, []);
  useEffect(() => {
    let active = true;
    let running = false;
    const tick = async () => {
      if (!active || running) return;
      running = true;
      try {
        await refresh();
      } catch (error) {
        if (active) setError(message(error));
      } finally {
        running = false;
      }
      if (active) setNow(Date.now());
    };
    void tick();
    void refreshDevices();
    const timer = setInterval(() => {
      if (document.visibilityState === "visible") void tick();
    }, 1000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, [refresh, refreshDevices]);
  useEffect(() => {
    setDraft(data?.projects.find((p) => p.id === selected) || null);
    setPreview(null);
    previewGeneration.current += 1;
  }, [selected]);
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("andromeda-theme", theme);
  }, [theme]);

  async function run(name: string, action: () => Promise<void>) {
    setBusy(name);
    setError("");
    setNotice("");
    try {
      await action();
      await refresh();
    } catch (error) {
      setError(message(error));
    } finally {
      setBusy("");
    }
  }
  async function addProject() {
    await run("project", async () => {
      const added = await invoke<Project | null>("choose_project");
      if (added) {
        await refresh();
        setSelected(added.id);
        setDraft(added);
        setNotice("Project added. Select a device, then connect your agent.");
      }
    });
  }
  async function saveProject() {
    if (!draft) return;
    await run("save", async () => {
      const saved = await invoke<Project>("save_project", {
        id: draft.id,
        serial: draft.serial,
        allowedTools: draft.allowed_tools,
        instructionFile: draft.instruction_file,
        enabled: draft.enabled,
      });
      if (saved.serial !== project?.serial) {
        previewGeneration.current += 1;
        setPreview(null);
      }
      setDraft(saved);
      setNotice(
        "Project access saved. Reconnect your agent to refresh its tool list.",
      );
    });
  }
  async function connect(client: string) {
    if (!project) return;
    await run("connect", async () => {
      const change = await invoke<Change>("preview_connection", {
        id: project.id,
        client,
      });
      setReplace(false);
      setDialog({ kind: "connection", client, change });
    });
  }
  async function editInstructions() {
    if (!project) return;
    await run("instructions", async () => {
      const document = await invoke<Instructions>("instructions", {
        id: project.id,
      });
      setDocumentText(document.content ?? document.starter);
      setDialog({ kind: "instructions", project, document });
    });
  }
  function toggleTool(name: string) {
    if (draft)
      setDraft({
        ...draft,
        allowed_tools: draft.allowed_tools.includes(name)
          ? draft.allowed_tools.filter((tool) => tool !== name)
          : [...draft.allowed_tools, name].sort(),
      });
  }
  async function stop() {
    try {
      const count = await invoke<number>("pause");
      setPreview(null);
      previewGeneration.current += 1;
      setNotice(
        `Input paused. ${count} managed request${count === 1 ? "" : "s"} cancelled.`,
      );
      await refresh();
    } catch (error) {
      setError(message(error));
    }
  }
  const activeApproval =
    dialog?.kind === "approval"
      ? data?.approvals.find((a) => a.id === dialog.approval.id)
      : undefined;

  return (
    <div className="app-shell">
      <a className="skip-link" href="#workspace">
        Skip to workspace
      </a>
      <header className="topbar">
        <div className="wordmark">
          <span className="brand-mark" aria-hidden="true">
            {<Star />}
          </span>
          ANDROMEDA<span className="edition">CONTROL / 01</span>
        </div>
        <div className="top-actions">
          {!!data?.approvals.length && (
            <button
              className="primary"
              onClick={() =>
                setDialog({ kind: "approval", approval: data.approvals[0] })
              }
            >
              Review {data.approvals.length}
            </button>
          )}
          <span className="local-label">ON YOUR COMPUTER</span>
          <button
            className="icon-button"
            onClick={() => setTheme(theme === "light" ? "dark" : "light")}
            aria-label={`Switch to ${theme === "light" ? "dark" : "light"} theme`}
          >
            <Icon name={theme === "light" ? "moon" : "sun"} />
          </button>
          <button className="stop-button" onClick={() => void stop()}>
            <Icon name="stop" />
            Stop &amp; pause
          </button>
        </div>
      </header>
      <main id="workspace">
        <section className="intro-grid" aria-label="Workspace overview">
          <div className="hero panel">
            <div className="hero-copy">
              <span className="eyebrow">YOUR AGENT, WITH REACH.</span>
              <h1>
                A phone.
                <br />
                On your terms.
              </h1>
              <p>
                Connect your agent to Android.
                <br />
                Choose the project. Set the boundaries.
              </p>
              <button
                className="primary"
                onClick={() => void addProject()}
                disabled={!!busy}
              >
                <Icon name="plus" />
                {busy === "project" ? "Choosing project…" : "Add a project"}
                <Icon name="arrow" />
              </button>
            </div>
            <Orbit />
          </div>
          <aside className="session panel">
            <span className="eyebrow">THE CONTROLS STAY WITH YOU</span>
            <div className="session-symbol" aria-hidden="true">
              {paused ? "Ⅱ" : "↗"}
            </div>
            <h2>{paused ? "Take your time." : "You’re in control."}</h2>
            <p>
              {paused
                ? "Input is paused. Your agent can use the observation tools you allow."
                : "Each input request needs your approval here before it reaches the phone."}
            </p>
            <button
              className="session-action"
              disabled={!!busy || !data}
              onClick={() =>
                paused
                  ? void run("resume", async () => {
                      await invoke("resume", {
                        generation: data?.pause_generation,
                      });
                      setNotice(
                        "Session ready. Input requests will appear for your approval.",
                      );
                    })
                  : void stop()
              }
            >
              {paused ? "Resume session" : "Pause session"}
              <Icon name={paused ? "arrow" : "stop"} />
            </button>
            <span className="status">
              <span className={`status-dot ${paused ? "" : "ready"}`} />
              {paused ? "INPUT PAUSED" : "AWAITING YOUR APPROVAL"}
            </span>
          </aside>
        </section>
        <div className="feedback">
          <p role="status" aria-live="polite">
            {notice || (!data && !error ? "Opening your workspace…" : "")}
          </p>
          {error && (
            <p role="alert" className="error-message">
              {error}
              <button className="text-button" onClick={() => setError("")}>
                Dismiss
              </button>
            </p>
          )}
        </div>
        {!!data?.approvals.length && (
          <section
            className="approval-banner"
            aria-label="Requests awaiting approval"
          >
            <div>
              <strong>
                {data.approvals.length} request
                {data.approvals.length === 1 ? "" : "s"} waiting
              </strong>
              <span>
                {data.approvals[0].project_name} ·{" "}
                {label(data.approvals[0].tool)}
              </span>
            </div>
            <button
              className="primary"
              onClick={() =>
                setDialog({ kind: "approval", approval: data.approvals[0] })
              }
            >
              Review request
              <Icon name="arrow" />
            </button>
          </section>
        )}
        <section className="workspace-grid">
          <aside className="projects panel">
            <div className="section-heading">
              <h2>Projects</h2>
              <span className="counter">
                {String(data?.projects.length || 0).padStart(2, "0")}
              </span>
            </div>
            <p className="muted small-text">A connection for each workspace.</p>
            <nav aria-label="Registered projects" className="project-list">
              {data?.projects.map((item) => (
                <button
                  key={item.id}
                  className={`project-item ${selected === item.id ? "selected" : ""}`}
                  aria-current={selected === item.id ? "page" : undefined}
                  disabled={!!busy || dirty}
                  onClick={() => {
                    setSelected(item.id);
                    setDraft(item);
                  }}
                >
                  <Icon name="folder" />
                  <span>
                    <strong>{item.name}</strong>
                    <span>
                      {item.enabled
                        ? "Project access enabled"
                        : "Access disabled"}
                    </span>
                  </span>
                  <span aria-hidden="true">↗</span>
                </button>
              ))}
            </nav>
            {!data?.projects.length && (
              <div className="empty-project">
                <Icon name="folder" size={32} />
                <p>
                  Your first project
                  <br />
                  starts here.
                </p>
                <button
                  className="text-button"
                  onClick={() => void addProject()}
                  disabled={!!busy}
                >
                  Choose a folder <span aria-hidden="true">↗</span>
                </button>
              </div>
            )}
            <div className="scope-note">
              <span className="eyebrow">A DEFINED SPACE</span>
              <p>
                Managed connections work inside their registered project
                directory. Other programs running as your OS user remain
                trusted.
              </p>
            </div>
          </aside>
          <div className="project-detail">
            {!draft || !project ? (
              <section className="panel welcome">
                <span className="eyebrow">01 / START WITH A PROJECT</span>
                <h2>
                  Make room for
                  <br />
                  your next idea.
                </h2>
                <p>
                  Choose the folder where you use Codex or Claude. Andromeda
                  will prepare a connection just for that project.
                </p>
                <div className="setup-steps">
                  <span>
                    <b>01</b>Choose a folder
                  </span>
                  <span>
                    <b>02</b>Select your phone
                  </span>
                  <span>
                    <b>03</b>Connect your agent
                  </span>
                </div>
                <div className="welcome-orbit">
                  <Orbit small />
                </div>
              </section>
            ) : (
              <>
                <section className="panel settings-panel">
                  <div className="section-heading">
                    <div>
                      <span className="eyebrow">PROJECT / ACCESS</span>
                      <h2>{project.name}</h2>
                    </div>
                    <span className="pill">
                      {project.enabled ? "ENABLED" : "DISABLED"}
                    </span>
                  </div>
                  <p className="project-path">{project.root}</p>
                  <form
                    onSubmit={(event) => {
                      event.preventDefault();
                      void saveProject();
                    }}
                  >
                    <fieldset disabled={!!busy || !paused}>
                      <legend className="sr-only">
                        Project access settings
                      </legend>
                      <div className="field-row">
                        <div className="field">
                          <label htmlFor="device">Android device</label>
                          <select
                            id="device"
                            value={draft.serial || ""}
                            onChange={(event) =>
                              setDraft({
                                ...draft,
                                serial: event.target.value || null,
                              })
                            }
                          >
                            <option value="">Choose a connected device</option>
                            {draft.serial &&
                              !deviceList.some(
                                (device) => device.serial === draft.serial,
                              ) && (
                                <option value={draft.serial}>
                                  {draft.serial} · disconnected
                                </option>
                              )}
                            {deviceList.map((device) => (
                              <option
                                key={device.serial}
                                value={device.serial}
                                disabled={device.state !== "device"}
                              >
                                {device.details
                                  .match(/model:(\S+)/)?.[1]
                                  ?.replace(/_/g, " ") || device.serial}{" "}
                                ·{" "}
                                {device.state === "device"
                                  ? "connected"
                                  : device.state}
                              </option>
                            ))}
                          </select>
                        </div>
                        <button
                          type="button"
                          className="secondary refresh-device"
                          onClick={() => void run("devices", refreshDevices)}
                        >
                          Refresh
                        </button>
                      </div>
                      {deviceError ? (
                        <p className="field-error">
                          {deviceError} Start ADB and reconnect your phone, then
                          refresh.
                        </p>
                      ) : (
                        deviceList.length === 0 && (
                          <p className="field-hint">
                            Connect your phone with USB debugging enabled, then
                            approve this computer on the phone.
                          </p>
                        )
                      )}
                      <div className="field">
                        <label htmlFor="instructions-path">
                          Project instruction file
                        </label>
                        <input
                          id="instructions-path"
                          value={draft.instruction_file}
                          onChange={(event) =>
                            setDraft({
                              ...draft,
                              instruction_file: event.target.value,
                            })
                          }
                          spellCheck={false}
                        />
                        <span className="field-hint">
                          A relative path inside this project. The agent
                          receives its contents when connecting.
                        </span>
                      </div>
                      <details className="tool-details">
                        <summary>
                          Allowed tools{" "}
                          <span>{draft.allowed_tools.length} enabled</span>
                        </summary>
                        <div className="tool-columns">
                          {[true, false].map((readOnly) => (
                            <div key={String(readOnly)}>
                              <h3>{readOnly ? "Observe" : "Interact"}</h3>
                              {data?.tools
                                .filter(
                                  (tool) =>
                                    tool.read_only === readOnly &&
                                    tool.name !== "adm_stop",
                                )
                                .map((tool) => (
                                  <label className="check-row" key={tool.name}>
                                    <input
                                      type="checkbox"
                                      checked={draft.allowed_tools.includes(
                                        tool.name,
                                      )}
                                      onChange={() => toggleTool(tool.name)}
                                    />
                                    <span>
                                      <strong>{label(tool.name)}</strong>
                                      <small>{tool.description}</small>
                                    </span>
                                  </label>
                                ))}
                            </div>
                          ))}
                        </div>
                        <p className="field-hint">
                          Stop is always available. Enabled input tools still
                          require your approval for each request.
                        </p>
                      </details>
                      <label className="check-row enable-project">
                        <input
                          type="checkbox"
                          checked={draft.enabled}
                          onChange={(event) =>
                            setDraft({
                              ...draft,
                              enabled: event.target.checked,
                            })
                          }
                        />
                        <span>Allow this project to connect</span>
                      </label>
                    </fieldset>
                    {!paused && (
                      <p className="field-hint">
                        Pause the session to edit project access.
                      </p>
                    )}
                    <div className="form-actions">
                      <button
                        className="primary"
                        type="submit"
                        disabled={!!busy || !paused || !dirty}
                      >
                        {busy === "save" ? "Saving…" : "Save access"}
                        <Icon name="arrow" />
                      </button>
                      <button
                        className="text-button"
                        type="button"
                        onClick={() => void editInstructions()}
                        disabled={!!busy || dirty}
                      >
                        Edit instructions
                      </button>
                      {dirty && (
                        <button
                          className="text-button"
                          type="button"
                          onClick={() => setDraft(project)}
                        >
                          Discard edits
                        </button>
                      )}
                    </div>
                  </form>
                </section>
                <section className="connection-grid">
                  <div className="panel agent-panel">
                    <span className="eyebrow">YOUR AGENT / CONNECT</span>
                    <h2>Meet on familiar ground.</h2>
                    <p className="connection-state" role="status">
                      {connection?.clients
                        ? `MCP connected · ${connection.clients} client${connection.clients === 1 ? "" : "s"}`
                        : "MCP disconnected"}
                    </p>
                    {connection?.initialization_error && (
                      <p className="field-error">
                        Connection failed: {connection.initialization_error}
                      </p>
                    )}
                    <p>
                      Preview the project configuration, then add Andromeda to
                      your agent.
                    </p>
                    <div className="agent-buttons">
                      <button
                        className="secondary"
                        onClick={() => void connect("codex")}
                        disabled={!!busy || dirty}
                      >
                        Connect Codex
                        <Icon name="arrow" />
                      </button>
                      <button
                        className="secondary"
                        onClick={() => void connect("claude")}
                        disabled={!!busy || dirty}
                      >
                        Connect Claude
                        <Icon name="arrow" />
                      </button>
                    </div>
                    <p className="field-hint">
                      Keep this app open while your agent is connected.
                    </p>
                  </div>
                  <div className="panel preview-panel">
                    <div className="section-heading">
                      <span className="eyebrow">DEVICE / PREVIEW</span>
                      <Icon name="phone" />
                    </div>
                    {preview && preview.serial === project.serial ? (
                      <>
                        <img
                          className="phone-preview"
                          src={preview.data_url}
                          alt="Most recently requested screenshot of the selected Android phone"
                        />
                        <p className="field-hint">
                          Snapshot · {preview.serial} ·{" "}
                          {Math.round(preview.elapsed_ms)} ms
                        </p>
                      </>
                    ) : (
                      <>
                        <h2>
                          A window
                          <br />
                          to your phone.
                        </h2>
                        <p>
                          Capture a screen when you’re ready to see it here.
                        </p>
                      </>
                    )}
                    <div className="preview-actions">
                      <button
                        className="secondary"
                        disabled={!!busy || !project.serial || dirty}
                        onClick={() =>
                          void run("preview", async () => {
                            const id = project.id;
                            const generation = ++previewGeneration.current;
                            const result = await invoke<Preview>(
                              "capture_preview",
                              { id },
                            );
                            if (
                              selectedRef.current === id &&
                              previewGeneration.current === generation
                            )
                              setPreview(result);
                          })
                        }
                      >
                        {busy === "preview"
                          ? "Capturing…"
                          : preview
                            ? "Refresh snapshot"
                            : "Show screen"}
                        <Icon name="arrow" />
                      </button>
                      {preview && (
                        <button
                          className="text-button"
                          onClick={() => {
                            previewGeneration.current += 1;
                            setPreview(null);
                          }}
                        >
                          Hide
                        </button>
                      )}
                    </div>
                  </div>
                </section>
              </>
            )}
          </div>
        </section>
        <footer className="footer">
          <span>
            ANDROMEDA <span aria-hidden="true">↗</span> A LITTLE MORE REACH.
          </span>
          <span>LOCAL CONTROL · VERSION 0.1.0 · PREVIEW</span>
        </footer>
      </main>
      {dialog && (
        <Modal
          onStop={() => void stop()}
          title={
            dialog.kind === "connection"
              ? `Connect ${dialog.client === "codex" ? "Codex" : "Claude"}`
              : dialog.kind === "instructions"
                ? "Project instructions"
                : "Review device input"
          }
          onClose={() => {
            if (!busy) setDialog(null);
          }}
        >
          {dialog.kind === "connection" && (
            <>
              <p>Review the Andromeda entry that will be written to:</p>
              <p className="path-block">{dialog.change.path}</p>
              {dialog.change.before && (
                <details>
                  <summary>Existing Andromeda entry</summary>
                  <pre>{dialog.change.before}</pre>
                </details>
              )}
              <h3>Proposed entry</h3>
              <pre>{dialog.change.after}</pre>
              <p className="field-hint">
                Your other client settings are preserved. Reopen the project or
                reconnect MCP afterward; your client may ask you to trust this
                connection.
              </p>
              {dialog.change.replaces_existing && (
                <label className="check-row">
                  <input
                    type="checkbox"
                    checked={replace}
                    onChange={(event) => setReplace(event.target.checked)}
                  />
                  <span>
                    Replace the existing Andromeda connection in this project
                  </span>
                </label>
              )}
              <div className="form-actions">
                <button
                  className="primary"
                  disabled={
                    !!busy || (dialog.change.replaces_existing && !replace)
                  }
                  onClick={() =>
                    void run("apply", async () => {
                      await invoke("apply_connection", {
                        id: dialog.change.id,
                        allowReplacement: replace,
                      });
                      setDialog(null);
                      setNotice(
                        "Connection saved. Reconnect MCP in your agent to use it.",
                      );
                    })
                  }
                >
                  {busy === "apply" ? "Saving…" : "Save connection"}
                </button>
                <button
                  className="secondary"
                  disabled={!!busy}
                  onClick={() => setDialog(null)}
                >
                  Cancel
                </button>
              </div>
            </>
          )}
          {dialog.kind === "instructions" && (
            <>
              <p className="path-block">{dialog.document.path}</p>
              <p>
                {dialog.document.content === null
                  ? "Start with these rules and adapt them to your project."
                  : "These instructions are supplied to the agent when it connects."}
              </p>
              <label className="sr-only" htmlFor="instruction-editor">
                Instruction file contents
              </label>
              <textarea
                id="instruction-editor"
                className="instruction-editor"
                value={documentText}
                onChange={(event) => setDocumentText(event.target.value)}
                spellCheck={false}
                disabled={!!busy || !paused}
              />
              <p className="field-hint">
                Reconnect your agent after saving to load the new instructions.
              </p>
              <div className="form-actions">
                <button
                  className="primary"
                  disabled={!!busy || !paused}
                  onClick={() =>
                    void run("write-instructions", async () => {
                      await invoke("save_instructions", {
                        id: dialog.project.id,
                        content: documentText,
                        previous: dialog.document.content,
                      });
                      setDialog(null);
                      setNotice("Project instructions saved.");
                    })
                  }
                >
                  Save instructions
                </button>
                <button
                  className="secondary"
                  disabled={!!busy}
                  onClick={() => setDialog(null)}
                >
                  Cancel
                </button>
              </div>
            </>
          )}
          {dialog.kind === "approval" && (
            <>
              <span className="eyebrow">{dialog.approval.project_name}</span>
              <p className="path-block">{dialog.approval.project_root}</p>
              <h3 className="request-name">{label(dialog.approval.tool)}</h3>
              <p>
                Device: <span className="mono">{dialog.approval.serial}</span>
              </p>
              <pre>{JSON.stringify(dialog.approval.arguments, null, 2)}</pre>
              <p className="field-hint">
                Approval applies only to this exact request. Check its effect
                before allowing it. Input already accepted by Android may finish
                after Stop.
              </p>
              <p role="status">
                {activeApproval
                  ? `Expires in ${Math.max(0, Math.ceil((activeApproval.expires_at_ms - now) / 1000))} seconds`
                  : "This request ended or was cancelled."}
              </p>
              <div className="form-actions">
                <button
                  className="primary"
                  disabled={!!busy || !activeApproval || paused}
                  onClick={() =>
                    void run("approve", async () => {
                      await invoke("decide", {
                        id: dialog.approval.id,
                        allow: true,
                      });
                      setDialog(null);
                      setNotice(
                        "Request approved. Check the resulting device state.",
                      );
                    })
                  }
                >
                  Allow once
                </button>
                <button
                  className="secondary"
                  disabled={!!busy || !activeApproval}
                  onClick={() =>
                    void run("deny", async () => {
                      await invoke("decide", {
                        id: dialog.approval.id,
                        allow: false,
                      });
                      setDialog(null);
                      setNotice("Request declined.");
                    })
                  }
                >
                  Decline
                </button>
              </div>
            </>
          )}
          {error && (
            <p role="alert" className="error-message">
              {error}
            </p>
          )}
        </Modal>
      )}
    </div>
  );
}
export default App;
