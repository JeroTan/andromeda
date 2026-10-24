import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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
type ProjectSession = {
  project_id: string;
  state: "running" | "paused" | "stopped";
  generation: number;
};
type BridgeStatus = {
  serial: string;
  connected: boolean;
  code: string | null;
  message: string;
};
type Snapshot = {
  projects: Project[];
  sessions: ProjectSession[];
  bridges: BridgeStatus[];
  pause_generation: number;
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
type PreviewFrame = Preview & {
  generation: string;
  encode_ms: number;
  sequence: number;
  width: number;
  height: number;
};
type PreviewError = {
  generation: string;
  serial: string;
  code: string;
  message: string;
};
type AgentClient = "codex" | "claude" | "opencode";
const clientNames: Record<AgentClient, string> = {
  codex: "Codex",
  claude: "Claude",
  opencode: "OpenCode",
};
type DialogState =
  | { kind: "connection"; client: AgentClient; change: Change }
  | { kind: "connection_result"; client: string; result: string }
  | { kind: "instructions"; project: Project; document: Instructions }
  | { kind: "remove"; project: Project };

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
    dialog.showModal();
    dialog.querySelector("h2")?.focus();
    return () => {
      dialog.close();
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
  const dialogTrigger = useRef<HTMLButtonElement | null>(null);
  const [documentText, setDocumentText] = useState("");
  const [replace, setReplace] = useState(false);
  const [preview, setPreview] = useState<Preview | null>(null);
  const [live, setLive] = useState(false);
  const [liveError, setLiveError] = useState("");
  const previewGeneration = useRef(0);
  const liveRef = useRef(false);
  const liveGeneration = useRef<string | null>(null);
  const previewEventsReady = useRef<Promise<void>>(Promise.resolve());
  const serialRef = useRef<string | null>(null);
  const [theme, setTheme] = useState(
    () => localStorage.getItem("andromeda-theme") || "light",
  );
  const selectedRef = useRef(selected);
  selectedRef.current = selected;
  const project = data?.projects.find((p) => p.id === selected);
  serialRef.current = project?.serial ?? null;
  const connection = data?.connections.find(
    (status) => status.project_id === selected,
  );
  const dirty = !!(
    draft &&
    project &&
    JSON.stringify(draft) !== JSON.stringify(project)
  );
  const session = data?.sessions.find((entry) => entry.project_id === selected);
  const paused = session?.state !== "running";
  const bridge = data?.bridges.find(
    (entry) => entry.serial === project?.serial,
  );
  const runningCount =
    data?.sessions.filter((entry) => entry.state === "running").length || 0;
  async function controlProject(action: ProjectSession["state"]) {
    if (!project || !session || !data) return;
    await run("session", async () => {
      await invoke("project_session", {
        id: project.id,
        action,
        generation: session.generation,
        globalGeneration: data.pause_generation,
      });
      setNotice("");
    });
  }

  const refresh = useCallback(async () => {
    const state = await invoke<Snapshot>("snapshot");
    setData(state);
    setSelected((current) =>
      state.projects.some((project) => project.id === current)
        ? current
        : state.projects[0]?.id || "",
    );
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
    liveRef.current = false;
    liveGeneration.current = null;
    setLive(false);
    setLiveError("");
    previewGeneration.current += 1;
    void invoke("stop_preview").catch(() => {});
  }, [selected]);
  useEffect(() => {
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const registrations: Array<Promise<void>> = [];
    function register<T>(name: string, handler: (payload: T) => void) {
      registrations.push(
        listen<T>(name, (event) => {
          if (!disposed) handler(event.payload);
        }).then((unlisten) => {
          if (disposed) unlisten();
          else unlisteners.push(unlisten);
        }),
      );
    }
    register<PreviewFrame>("preview-frame", (frame) => {
      if (
        !liveRef.current ||
        frame.generation !== liveGeneration.current ||
        frame.serial !== serialRef.current
      )
        return;
      setPreview(frame);
      setLiveError("");
      if (frame.sequence === 1)
        setNotice("Live preview started. It runs until you stop it.");
    });
    register<PreviewError>("preview-error", (error) => {
      if (
        !liveRef.current ||
        error.generation !== liveGeneration.current ||
        error.serial !== serialRef.current
      )
        return;
      liveRef.current = false;
      liveGeneration.current = null;
      setLive(false);
      setNotice("");
      setLiveError(error.message);
    });
    previewEventsReady.current = Promise.all(registrations).then(() => {});
    void previewEventsReady.current.catch((error) => {
      if (!disposed) setError(message(error));
    });
    return () => {
      disposed = true;
      for (const unlisten of unlisteners) unlisten();
    };
  }, []);
  useEffect(
    () => () => {
      liveGeneration.current = null;
      liveRef.current = false;
      void invoke("stop_preview").catch(() => {});
    },
    [],
  );
  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    localStorage.setItem("andromeda-theme", theme);
  }, [theme]);

  useEffect(() => {
    if (dialog || busy) return;
    // Async setup disables the trigger before the dialog mounts. Restore it
    // only after the dialog is removed and the triggering control is enabled.
    const trigger = dialogTrigger.current;
    dialogTrigger.current = null;
    if (trigger?.isConnected && !trigger.disabled) trigger.focus();
  }, [dialog, busy]);

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
        liveRef.current = false;
        liveGeneration.current = null;
        setLive(false);
        setLiveError("");
      }
      setDraft(saved);
      setNotice(
        "Project access saved. Reconnect your agent to refresh its tool list.",
      );
    });
  }
  async function connect(client: AgentClient) {
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
  async function startLive() {
    if (!project?.serial) return;
    const generation = Array.from(
      crypto.getRandomValues(new Uint8Array(16)),
      (byte) => byte.toString(16).padStart(2, "0"),
    ).join("");
    setBusy("live");
    setError("");
    setNotice("Starting live preview…");
    setPreview(null);
    setLiveError("");
    // Accept matching errors/frames before native startup can emit an event.
    liveGeneration.current = generation;
    liveRef.current = true;
    setLive(true);
    try {
      await previewEventsReady.current;
      if (liveGeneration.current !== generation || !liveRef.current) return;
      await invoke("start_preview", { id: project.id, generation });
    } catch (error) {
      if (liveGeneration.current === generation) {
        liveGeneration.current = null;
        liveRef.current = false;
        setLive(false);
        setNotice("");
        setError(message(error));
      }
    } finally {
      setBusy("");
    }
  }
  async function stopLive() {
    liveGeneration.current = null;
    liveRef.current = false;
    setLive(false);
    await invoke("stop_preview").catch(() => {});
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
    setNotice("Stopping all projects…");
    liveGeneration.current = null;
    liveRef.current = false;
    setLive(false);
    setPreview(null);
    previewGeneration.current += 1;
    try {
      const count = await invoke<number>("emergency_stop");
      setNotice(
        `All projects stopped. ${count} managed request${count === 1 ? "" : "s"} cancelled.`,
      );
      await refresh();
    } catch (error) {
      setError(message(error));
    }
  }
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
            Stop all projects
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
            <span className="eyebrow">PROJECT SESSIONS</span>
            <div className="session-symbol" aria-hidden="true">
              ↗
            </div>
            <h2>
              {runningCount
                ? `${runningCount} running.`
                : "Ready when you are."}
            </h2>
            <p>
              Start a project once. Its allowed tools run without repeated
              approval prompts. Each project has its own Pause and Stop.
            </p>
            <span className="status">
              {runningCount ? "AUTOMATION RUNNING" : "NO RUNNING PROJECTS"}
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
                        ? data?.sessions.find(
                            (entry) => entry.project_id === item.id,
                          )?.state || "stopped"
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
                  <div
                    className="project-session"
                    aria-label={`${project.name} session`}
                  >
                    <div role="status" aria-live="polite">
                      <strong>
                        {session?.state === "running"
                          ? "Running · allowed actions execute automatically"
                          : session?.state === "paused"
                            ? "Paused · resume when ready"
                            : "Stopped · start this project when ready"}
                      </strong>
                      <p className="field-hint">
                        Pause and Stop block input. Allowed observation tools
                        remain available.
                      </p>
                      {dirty && paused && (
                        <p className="field-hint">
                          You have unsaved access changes. Save access here,
                          then start the project.
                        </p>
                      )}
                      <p className="field-hint">
                        {bridge?.connected
                          ? "Phone and desktop connected · session status synchronized"
                          : bridge?.code === "companion_not_paired"
                            ? "USB controls available. Pair the optional phone bridge once for synchronized phone controls, multi-touch and Unicode."
                            : bridge?.message || "Checking phone bridge…"}
                      </p>
                    </div>
                    <div className="form-actions">
                      <button
                        className="primary"
                        disabled={
                          !!busy ||
                          (paused &&
                            !dirty &&
                            (!project.enabled || !project.serial))
                        }
                        onClick={() =>
                          dirty && paused
                            ? void saveProject()
                            : void controlProject(paused ? "running" : "paused")
                        }
                      >
                        {dirty && paused
                          ? "Save access"
                          : session?.state === "running"
                            ? "Pause project"
                            : session?.state === "paused"
                              ? "Resume project"
                              : "Start project"}
                      </button>
                      <button
                        className="secondary"
                        disabled={!!busy || session?.state === "stopped"}
                        onClick={() => void controlProject("stopped")}
                      >
                        Stop project
                      </button>
                    </div>
                  </div>
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
                          Start this project once to run these allowed tools
                          automatically. Pause or Stop affects only this
                          project.
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
                        Pause this project to edit or remove its access.
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
                        onClick={(event) => {
                          dialogTrigger.current = event.currentTarget;
                          void editInstructions();
                        }}
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
                      <button
                        className="text-button danger"
                        type="button"
                        onClick={(event) => {
                          dialogTrigger.current = event.currentTarget;
                          setDialog({ kind: "remove", project });
                        }}
                        disabled={!!busy || dirty || !paused}
                        title={
                          !paused
                            ? "Pause this project before removing it"
                            : undefined
                        }
                      >
                        Remove project
                      </button>
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
                        : "Waiting for an agent to load Andromeda"}
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
                        onClick={(event) => {
                          dialogTrigger.current = event.currentTarget;
                          void connect("codex");
                        }}
                        disabled={!!busy || dirty}
                      >
                        Connect Codex
                        <Icon name="arrow" />
                      </button>
                      <button
                        className="secondary"
                        onClick={(event) => {
                          dialogTrigger.current = event.currentTarget;
                          void connect("claude");
                        }}
                        disabled={!!busy || dirty}
                      >
                        Connect Claude
                        <Icon name="arrow" />
                      </button>
                      <button
                        className="secondary"
                        onClick={(event) => {
                          dialogTrigger.current = event.currentTarget;
                          void connect("opencode");
                        }}
                        disabled={!!busy || dirty}
                      >
                        Connect OpenCode
                        <Icon name="arrow" />
                      </button>
                    </div>
                    <p className="field-hint">
                      Keep this app open while your agent is connected.
                    </p>
                    <details className="tool-details">
                      <summary>Optional phone bridge</summary>
                      <p className="field-hint">
                        For multi-touch and Unicode, open Andromeda on the
                        phone, enable its accessibility service, then choose
                        Pair this computer. Once paired, project controls stay
                        synchronized automatically.
                      </p>
                      <div className="form-actions">
                        <button
                          className="secondary"
                          disabled={
                            !!busy || !paused || !project.serial || dirty
                          }
                          onClick={() =>
                            void run("pair", async () => {
                              await invoke("pair_companion", {
                                id: project.id,
                              });
                              setNotice(
                                "Phone bridge paired. Start the project here; no separate phone Start is needed.",
                              );
                            })
                          }
                        >
                          Pair this phone
                        </button>
                      </div>
                      <p className="field-hint" role="status">
                        {bridge?.message || "Checking phone bridge…"}
                      </p>
                    </details>
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
                          alt={
                            live
                              ? "Live preview of the selected Android phone"
                              : "Most recently requested screenshot of the selected Android phone"
                          }
                        />
                        <p className="field-hint" role="status">
                          {live ? "Live" : "Snapshot"} · {preview.serial} ·{" "}
                          {live
                            ? `${(1000 / Math.max(preview.elapsed_ms, 1)).toFixed(2)} fps`
                            : `${Math.round(preview.elapsed_ms)} ms`}
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
                          Capture a screen on request, or start an explicit live
                          preview. Nothing is saved.
                        </p>
                      </>
                    )}
                    {liveError && (
                      <p className="field-error" role="alert">
                        {liveError}
                      </p>
                    )}
                    <div className="preview-actions">
                      {!live && (
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
                      )}
                      {live ? (
                        <button
                          className="secondary"
                          aria-pressed="true"
                          onClick={() => void stopLive()}
                        >
                          Stop live preview
                        </button>
                      ) : (
                        <button
                          className="secondary"
                          aria-pressed="false"
                          disabled={!!busy || !project.serial || dirty}
                          onClick={() => void startLive()}
                        >
                          {busy === "live" ? "Starting…" : "Start live preview"}
                          <Icon name="arrow" />
                        </button>
                      )}
                      {preview && (
                        <button
                          className="text-button"
                          onClick={() => {
                            if (live) void stopLive();
                            previewGeneration.current += 1;
                            setPreview(null);
                            setLiveError("");
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
          <span>LOCAL CONTROL · VERSION 0.1.2 · PREVIEW</span>
        </footer>
      </main>
      {dialog && (
        <Modal
          onStop={() => void stop()}
          title={
            dialog.kind === "connection"
              ? `Connect ${clientNames[dialog.client]}`
              : dialog.kind === "connection_result"
                ? "Claude connection configured"
                : dialog.kind === "instructions"
                  ? "Project instructions"
                  : dialog.kind === "remove"
                    ? "Remove project"
                    : "Review device input"
          }
          onClose={() => {
            if (!busy) setDialog(null);
          }}
        >
          {dialog.kind === "connection" && (
            <>
              <p>
                {dialog.client === "claude"
                  ? "Connect Claude to Andromeda for this project:"
                  : "Review the Andromeda entry that will be written to:"}
              </p>
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
                {dialog.client === "claude"
                  ? "This registers Andromeda in your private Claude setup for this project and checks the connection. Other projects and servers are preserved."
                  : dialog.client === "opencode"
                    ? "For OpenCode 2. Saves this project's connection while preserving your settings and comments. Reopen OpenCode in this project to load Andromeda."
                    : "Your other client settings are preserved. Reopen the project or reconnect MCP afterward; your client may ask you to trust this connection."}
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
                      const result = await invoke<string>("apply_connection", {
                        id: dialog.change.id,
                        allowReplacement: replace,
                      });
                      if (dialog.client === "claude") {
                        setDialog({
                          kind: "connection_result",
                          client: dialog.client,
                          result,
                        });
                      } else {
                        setDialog(null);
                      }
                      setNotice(result);
                    })
                  }
                >
                  {busy === "apply"
                    ? dialog.client === "claude"
                      ? "Connecting…"
                      : "Saving…"
                    : dialog.client === "claude"
                      ? "Connect this project"
                      : "Save connection"}
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
          {dialog.kind === "connection_result" && (
            <>
              <p role="status">{dialog.result}</p>
              <h3>Already have a Claude chat open?</h3>
              <p>
                That conversation keeps its existing tool list. Reconnect
                Andromeda from Claude's MCP panel or reopen the conversation to
                load the newly configured tools.
              </p>
              <p className="field-hint">
                The add command registers a server; it cannot refresh a running
                Claude conversation. No further Andromeda approval is needed.
              </p>
              <button className="primary" onClick={() => setDialog(null)}>
                Done
              </button>
            </>
          )}
          {dialog.kind === "remove" && (
            <>
              <p>
                Remove <strong>{dialog.project.name}</strong> from Andromeda?
              </p>
              <p className="path-block">{dialog.project.root}</p>
              <p className="field-hint">
                This removes the project registration and stops its live
                preview. Your project files stay on disk. Existing Codex or
                Claude configuration is not changed.
              </p>
              <div className="form-actions">
                <button
                  className="danger-button"
                  disabled={!!busy || !paused}
                  onClick={() =>
                    void run("remove", async () => {
                      const id = dialog.project.id;
                      await invoke("remove_project", { id });
                      if (selectedRef.current === id) {
                        setSelected("");
                        setDraft(null);
                      }
                      previewGeneration.current += 1;
                      setPreview(null);
                      liveGeneration.current = null;
                      liveRef.current = false;
                      setLive(false);
                      setLiveError("");
                      setDialog(null);
                      setNotice(
                        "Project removed from Andromeda. Its files were not changed.",
                      );
                    })
                  }
                >
                  {busy === "remove" ? "Removing…" : "Remove project"}
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
