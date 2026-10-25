# Andromeda 🌌

### Give your AI agent a phone.

**An open-source, high-performance Android control interface for your favorite AI agents. Built with Rust. Powered by MCP.**

Your phone is full of things you don't want to do manually. Andromeda aims to let your AI agent handle them.

Connect your Android device. Connect your favorite AI agent. Give it instructions.

**You decide what needs to happen. Your agent figures out how.**

> 🚧 **Status: Planning / Pre-alpha**
>
> Andromeda is currently a development concept. Features and integrations below describe the planned product, not a released implementation.

---

## 🤖 Bring your favorite AI agent

Andromeda is designed to work with any AI client that supports the Model Context Protocol (MCP).

**Target integrations:**

- **Claude Code** — Anthropic
- **OpenAI Codex** — OpenAI
- **Cursor** — AI-powered development environment
- **Other MCP-compatible agents**

One Android interface. Multiple AI agents. No need to build separate automation scripts for every application.

*Compatibility will be tested and documented as integrations are implemented. Andromeda is an independent open-source project, not affiliated with these providers.*

---

## What if your AI could actually use your phone?

Not just answer questions about your phone.

Actually **see the screen, tap buttons, swipe, type, navigate, and complete tasks**.

### 🎮 Tired of games that don't respect your time?

Ever wanted to skip endless dialogue, summarize a story, or delegate repetitive tasks?

Imagine telling your agent:

> "Navigate this story section and summarize the important plot points afterward."

Or:

> "Inspect the available daily activities and tell me which ones still need attention."

Andromeda aims to expose general-purpose device controls rather than game-specific automation.

*Game automation must comply with applicable game rules. Using automated agents in competitive or restricted environments can result in penalties or account bans.*

### 📱 Doomscrolling again?

What if your agent could review the content you care about instead?

> "Go through my selected feed, identify the interesting posts, and give me a five-minute summary."

Less mindless scrolling. More useful information.

### 💼 Your phone could actually help you work

You have mobile-only workflows, repetitive menus, and tasks that feel unnecessarily complicated.

> "Open my work app, collect the status updates, and summarize everything that needs my attention."

Make your phone a useful extension of your workflow—not a source of more work.

### 💘 Dating-app fatigue?

What if an agent helped you review profiles according to your actual preferences?

> "Help me identify profiles that match my stated interests. I'll decide whom to interact with."

Review and organization are one thing. Mass-swiping, impersonation, and automated messaging may violate platform rules. Andromeda should support human approval for these interactions.

### 📈 Monitoring markets from your phone?

> "Check my portfolio, summarize the changes, and show me the biggest risks."

Let your agent gather information and prepare decisions. Financial transactions and account-changing actions should require explicit human confirmation.

### ♟️ Could an AI play games using real screen controls?

Potentially, yes.

Touch interaction is a general capability. Andromeda could support game AI research, permitted single-player experiments, chess puzzles, or custom test environments.

It isn't intended to facilitate competitive cheating, unauthorized rank boosting, or manipulation of real players.

### And that's only the beginning.

If an Android application exposes an accessible interface, Andromeda may be able to interact with it.

**No application-specific scripts required in principle. Just tools, visual feedback, and an agent capable of reasoning about the task.**

Actual capabilities will depend on device permissions, application restrictions, and model reliability.

---

## How Andromeda works

Andromeda separates AI reasoning from device interaction.

The AI decides **what to do**.

Andromeda provides **the tools to do it**.

```text
┌───────────────────────────────┐
│           AI Agent            │
│ Claude Code / Codex / Others  │
│                               │
│ Instructions • Reasoning      │
│ Persona • Task Planning       │
└──────────────┬────────────────┘
               │
               │ MCP
               ▼
┌───────────────────────────────┐
│       ANDROMEDA SERVER        │
│             Rust              │
│                               │
│  adm_get_view                 │
│  adm_touch                    │
│  adm_touch_many               │
│  adm_swipe                    │
│  adm_swipe_many               │
│  adm_multi_touch              │
│  adm_type_text                │
│  adm_get_ui_tree              │
│  adm_batch_actions            │
│  ...                          │
└──────────────┬────────────────┘
               │
               │ USB / ADB
               ▼
┌───────────────────────────────┐
│         Android Device        │
│                               │
│   Screenshots • Touch Input   │
│   Apps • Gestures • UI Data   │
└───────────────────────────────┘
```

### The interaction loop

1. The agent observes the current screen.
2. The agent interprets the available information.
3. The agent decides on an action.
4. Andromeda executes that action on the device.
5. The agent observes the result.
6. The process repeats until the objective is verified or execution stops.

The Rust controller is intentionally application-agnostic.

---

## Planned capabilities

### 👁️ Screen and device observation

| Tool | Description |
|---|---|
| `adm_get_view` | Capture the current screen as an image |
| `adm_get_ui_tree` | Retrieve accessible UI information |
| `adm_get_device_info` | Resolution, orientation and device details |
| `adm_list_devices` | Enumerate connected Android devices |
| `adm_list_apps` | List applications available on the device |

### 👆 Touch and interaction

| Tool | Description |
|---|---|
| `adm_touch` | Tap a screen coordinate |
| `adm_touch_many` | Execute multiple tap events |
| `adm_swipe` | Swipe or drag between coordinates |
| `adm_swipe_many` | Execute multiple swipe gestures |
| `adm_multi_touch` | Execute synchronized multi-finger gestures |
| `adm_press_key` | Press Android navigation or hardware keys |
| `adm_type_text` | Enter text through an available input method |

`adm_touch_many` and `adm_swipe_many` will support clearly defined timing modes. True simultaneous multi-touch will use `adm_multi_touch` or an equivalent gesture backend.

### ⚙️ Applications and task execution

| Tool | Description |
|---|---|
| `adm_launch_app` | Launch an installed application |
| `adm_open_uri` | Open a supported Android URI |
| `adm_wait_for` | Wait for a supported UI or visual condition |
| `adm_batch_actions` | Execute coordinated action sequences |
| `adm_stop` | Cancel active automation |

**Naming convention:** Every public MCP tool must begin with `adm_`.

### Tool behavior requirements

Every tool should provide structured, machine-readable results with:

- Success or failure status
- Relevant return data
- Descriptive errors
- Execution timing where applicable
- Cancellation and timeout support

Actions should use a defined screen coordinate system and handle orientation changes safely.

---

## Getting started

### Requirements (planned)

- Android device with USB debugging enabled
- Windows, macOS, or Linux computer
- Android Debug Bridge (ADB)
- Rust toolchain for building from source
- An MCP-compatible AI client

Root access should not be necessary for basic screen capture and ordinary interaction.

Advanced gesture injection or accessibility inspection may require an optional Android companion application with user-granted permissions.

### Example workflow

Once Andromeda's first executable is available:

1. Connect your Android device through USB.
2. Authorize USB debugging on the device.
3. Launch the Andromeda MCP server.
4. Register Andromeda in Claude Code, Codex, or another supported MCP client.
5. Give your agent an instruction.

Example prompt:

> "Check what's currently displayed on my phone. Open Settings, enable dark mode, and verify the result."

The agent should use Andromeda's tools to observe, act, and verify rather than assuming that input execution means the task succeeded.

*Install commands and executable configuration will be added when a working release is available.*

---

## Technical architecture

### Product interface requirements added on 2026-10-22

The main Andromeda interface runs on the user's Mac or PC alongside Codex and Claude. It creates and manages the MCP connection used by chats. The optional phone application is a permission and gesture bridge; its setup flow is part of the product and must receive the same visual and usability care as the desktop interface.

**Desktop setup and project access**

- Select a project directory, Android device and allowed tool groups.
- Preview and apply project-local Codex and Claude MCP configuration while preserving existing settings.
- Register project access explicitly and reject managed-server startup from an unrelated directory. Resolve canonical paths and symlinks when checking scope.
- Load a user-selected instruction file within the project so the agent receives the project's device-use rules. Provide an editable starter file.
- Keep credentials in private application storage. Configuration copied to another project must not silently authorize that project.
- Show device and MCP connection states, an explicitly enabled screen preview, session state and an accessible stop control.
- Explain how project scoping works and where the operating-system user remains the trust boundary.
- Provide a tested npm launch command and downloadable desktop executables/packages that open the interface with minimal setup. The user clarified on 2026-10-24: the Lynx phone UI ships as an Android APK, macOS ships a DMG, and Windows ships an EXE installer. npm (`@jrw/andromeda`) is an optional launch route, not a replacement for native installers.

**Visual direction**

Use the user's supplied editorial reference as inspiration for an original Andromeda identity:

- Bold, oversized sans-serif headings, readable body text and a clear hierarchy.
- An off-white or pale-lavender canvas, strong ink-colored text, thin borders and rounded modular panels.
- Restrained coral, peach, mint and sky-blue gradients for focal elements and primary actions.
- Original orbital paths, constellation marks or abstract celestial artwork to express the Andromeda name.
- Spacious layouts with useful device/project information. Give each screen one clear primary action.
- Build corresponding light and dark themes, with contrast checked independently.

The reference's typography, panel structure and soft color treatment should guide the interface. Use original product copy, artwork and identity assets.

**Phone bridge experience**

- Use a concise, polished flow for accessibility permission, pairing and session status.
- Keep sessions open until the user stops them by default. Provide a Settings button, clickable settings sections and a Session section containing an optional session timer (Off by default, 15 minutes, 30 minutes or 1 hour). Persist the choice and apply it to the next session. Keep Stop accessible throughout settings navigation.
- Explain why Android requires each permission at the point it is needed. Detect existing permission, saved pairing and active session before choosing a setup step. Refresh on return to the app. Distinguish an enabled permission from a temporarily disconnected service; never reopen permission settings automatically or repeat completed setup.
- Keep stop/disconnect readily available, with clear disabled, disconnected, unauthorized, expired and error states.
- Put development-only gesture diagnostics behind a development entry point.
- Respect system text scaling, screen readers, native back navigation and safe areas. Controls must meet Android's 48 dp touch target guidance.
- Verify small-screen and landscape layouts, reduced motion, keyboard/focus behavior where applicable, and both color themes.

**UI technology evaluation**

Evaluate Lynx/ReactLynx for a TypeScript presentation layer, including native desktop integration and Android native-module support. Rust remains responsible for MCP, device control, permissions, cancellation and performance-sensitive work. Record the UI framework decision with primary sources and a build/launch check; do not assume that a framework's renderer language is Rust or that desktop rendering alone supplies a complete desktop application framework.

**Release acceptance additions**

- Desktop project registration, configuration preview/application, instruction loading and project isolation are implemented and verified.
- Desktop and phone interfaces match the approved visual direction and pass relevant accessibility and responsive checks.
- The documented one-command launcher and desktop packages are tested against the release artifacts.
- Full automated suites and platform checks run in GitHub PR workflows. Local checks focus on individual changes, debugging and tests requiring the connected phone. Android Studio/emulators are available for testing.

Andromeda will be built primarily in **Rust**, prioritizing low overhead, predictable execution, and portability.

### Planned technology stack

| Component | Technology |
|---|---|
| Core language | Rust |
| Asynchronous runtime | Tokio |
| MCP implementation | Rust MCP SDK (`rmcp`) |
| Android connectivity | ADB |
| Screen capture | ADB initially; persistent streaming later |
| Screen streaming | scrcpy-based backend evaluation |
| Complex gesture injection | Android companion application |
| UI accessibility | Android AccessibilityService |
| Serialization | Serde / JSON |
| Image processing | Lightweight decoding and optional OpenCV integration |

The device layer should be reusable independently of MCP so alternative interfaces can be added without rewriting the controller.

### Internal components

```text
andromeda/
├── README.md
├── LICENSE
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── mcp/
│   ├── device/
│   ├── capture/
│   ├── input/
│   ├── accessibility/
│   ├── actions/
│   ├── safety/
│   └── errors/
├── android-companion/
├── benchmarks/
├── tests/
├── examples/
└── docs/
```

This is a proposed layout, not an existing repository structure.

---

## Performance philosophy

**Fast is good. Reliable and fast is better.**

Andromeda should aim to compete with existing Android MCP implementations on performance and developer experience.

We should not claim to be the fastest without reproducible evidence.

### Optimization priorities

1. **Persistent connections:** Minimize repeated ADB process startup.
2. **Efficient screen capture:** Avoid unnecessary full-resolution captures.
3. **Image compression:** Reduce unnecessary image-transfer and AI context costs.
4. **Action batching:** Reduce round trips for predetermined sequences.
5. **Precise gesture timing:** Support accurate durations and concurrent pointers.
6. **Asynchronous execution:** Keep the server responsive during long operations.
7. **Reliable recovery:** Return actionable errors rather than silently failing.

### Benchmark metrics

- Screenshot latency (p50 / p95)
- Input dispatch latency (p50 / p95)
- Screenshot-to-action turnaround
- Effective screen-stream frame rate
- Memory and CPU usage
- Gesture accuracy
- Task completion and recovery rate
- Model/tool-call overhead

Comparisons should use the same devices, resolutions, workloads, and test conditions.

Optimization claims will be published alongside benchmarks.

---

## Agent instructions and personas

Andromeda should not hardcode the behavior of a particular AI model.

Instead, developers can supply their own instructions or persona configuration through their preferred agent environment.

Example:

```md
# Device Agent Instructions

You are controlling an Android device
through Andromeda MCP tools.

Operating rules:
- Inspect the screen before interacting.
- Never assume a tap succeeded.
- Observe the result of important actions.
- Prefer accessible UI elements when reliable.
- Use visual coordinates when necessary.
- Recover from unexpected screens.
- Stop and report when uncertain.
- Ask for approval before sensitive operations.
- Verify completion before reporting success.
```

Future versions may provide reusable instruction packs and task profiles.

**The core tool interface remains model-independent.**

---

## Development roadmap

### Phase 1 — Core Android MCP server

**Goal:** Give an AI agent basic eyes and fingers.

- [ ] Initialize Rust project
- [ ] Integrate MCP server transport
- [ ] Implement device discovery
- [ ] Implement `adm_get_device_info`
- [ ] Implement `adm_get_view`
- [ ] Implement `adm_touch`
- [ ] Implement `adm_swipe`
- [ ] Implement `adm_press_key`
- [ ] Implement basic `adm_type_text`
- [ ] Test an end-to-end interaction in Claude Code
- [ ] Test an end-to-end interaction in Codex

**Completion criterion:** An MCP agent can observe a real Android screen, perform actions, and confirm that a setting changed.

### Phase 2 — Advanced interaction

**Goal:** Provide reliable human-like gesture primitives.

- [ ] Implement `adm_touch_many`
- [ ] Implement `adm_swipe_many`
- [ ] Implement true simultaneous `adm_multi_touch`
- [ ] Evaluate optional companion APK
- [ ] Implement `adm_batch_actions`
- [ ] Implement `adm_stop`
- [ ] Add action timeouts
- [ ] Add coordinate and timing validation
- [ ] Add gesture integration tests

**Completion criterion:** Coordinated gestures work reliably on supported physical devices.

### Phase 3 — Perception and reliability

**Goal:** Reduce the chance of agents acting on stale or incorrect information.

- [ ] Implement `adm_get_ui_tree`
- [ ] Implement `adm_wait_for`
- [ ] Improve screen streaming
- [ ] Add screenshot scaling and compression options
- [ ] Handle screen rotation
- [ ] Handle USB disconnects and reconnects
- [ ] Add explicit device selection
- [ ] Add task cancellation and action logs
- [ ] Add permission and approval boundaries

**Completion criterion:** The controller behaves predictably when applications change state or unexpected conditions occur.

### Phase 4 — Performance

**Goal:** Demonstrate measurable improvements.

- [ ] Create reproducible benchmark suite
- [ ] Measure ADB screenshot performance
- [ ] Benchmark persistent screen capture
- [ ] Optimize input dispatch
- [ ] Reduce unnecessary allocations and copying
- [ ] Compare representative open-source alternatives
- [ ] Publish measurements and limitations

**Completion criterion:** Benchmark results can be reproduced by other developers.

### Phase 5 — Open-source release

**Goal:** Make Andromeda easy to install, understand, and contribute to.

- [ ] Verify platform compatibility
- [ ] Publish installation guide
- [ ] Publish Claude Code integration guide
- [ ] Publish Codex integration guide
- [ ] Publish API reference
- [ ] Provide sample prompts and demonstrations
- [ ] Set up GitHub Actions CI
- [ ] Add CONTRIBUTING.md
- [ ] Add SECURITY.md
- [ ] Choose an appropriate open-source license
- [ ] Release version `v0.1.0`

---

## Design principles

**1. Agent-agnostic**

Andromeda should never depend on a single model vendor.

**2. Application-agnostic**

The core controller should expose device primitives instead of hardcoded application scripts.

**3. Performance-conscious**

Latency, resource consumption, and unnecessary tool calls matter.

**4. Observable**

Every meaningful action should be verifiable.

**5. Interruptible**

Users must be able to stop device automation.

**6. Local-first**

Prefer explicit, authenticated, user-controlled connections. Avoid exposing device-control tools to untrusted networks.

**7. Permission-aware**

Accessibility, sensitive information, and account-changing operations require careful permission boundaries.

---

## Security, ethics, and disclaimers

Andromeda is a general-purpose interface for authorized Android device interaction.

It does not guarantee that every application, game, or protected interface can be automated.

Users are responsible for complying with applicable laws, application terms, and platform policies.

### Important limitations

- **Game automation:** Some games prohibit bots, scripts, automated gameplay, or third-party tools. Account penalties may apply.
- **Competitive integrity:** Andromeda is not intended for cheating, match manipulation, unauthorized ranking boosts, or deceptive multiplayer activity.
- **Financial actions:** Trading, purchasing, and transferring funds should require explicit human approval.
- **Social platforms:** Avoid spam, nonconsensual interactions, impersonation, or mass account manipulation.
- **Workplace use:** Automate legitimate tasks, not fabricated attendance, engagement, or work activity.
- **Privacy:** Screenshots and accessibility data may contain passwords, messages, account details, and other sensitive information.
- **Device security:** USB debugging and accessibility privileges can expose powerful device capabilities. Enable them only for trusted software and connections.

### Security goals

- Local-only MCP server by default
- Explicit device authorization
- Sensitive-action approval controls
- Minimal permission requirements
- Cancellation and emergency-stop support
- No screenshot retention by default
- No hidden background monitoring
- No credential harvesting or security-control bypasses

Agent autonomy must remain subordinate to user authorization and safety boundaries.

---

## Contributing

Andromeda is intended to become a community-driven open-source project.

Contributions may include:

- Rust performance optimization
- Android compatibility fixes
- MCP client integrations
- Gesture and accessibility improvements
- Benchmark tooling
- Documentation
- Testing on additional Android devices

Before contributing, review the project's contribution guidelines and license once published.

Existing open-source projects can inform Andromeda's feature design, but reused source code must comply with its original license and attribution requirements.

---

## License

**To be decided before the first public release.**

MIT and Apache-2.0 are being considered for original code, subject to dependency and third-party license review.

---

### Andromeda 🌌

**Your phone. Your agent. Your instructions.**

Stop building one automation for every app.

Build an interface that lets your AI figure out the rest.

*Built with Rust. Designed for MCP. Created for developers who want to give their AI agents a way to interact with the real world.*

### Automation additions requested on 2026-10-23

- Add a bounded long-view tool for scrolling lists: explicit viewport and swipe, page/deadline limits, settling delay, repeat detection, and readable page mapping. Treat scrolling as input requiring authorization/approval. Honor Stop, cancellation and rotation changes. Do not claim that a bounded scan sees an entire unbounded list or that older image tiles are current tap coordinates.
- Expose and document sequential tap/swipe batches with configurable gaps and per-step pauses. Execute predetermined timing locally to avoid model round trips; prevalidate all actions and stop on failure. Existing `adm_touch_many`, `adm_swipe_many` and `adm_batch_actions` provide this behavior.
- Separate device capture/input latency, MCP/client overhead and model thinking time when measuring interactive tasks. Retain timing samples and verified outcomes without publishing account screenshots.
- Investigate adaptive held gestures as a separate controller capability: one owner per device, explicit bounded gesture lease, continuation updates, a local expiry/watchdog and release on Stop/disconnect. Planning agents may propose updates; the local controller owns precise timing and arbitration. Do not use model or subagent response time as an input-timing guarantee. This remains a planned capability until implemented and tested through the public companion protocol.

- Support compressed screen output using lossy/lossless WebP and an optional AVIF encoder. Measure payload size and processing cost on the same source. Offer automatic selection among compatible PNG/JPEG/WebP encodings, reporting the chosen MIME type; never choose AVIF automatically for ordinary vision clients. Preserve image/native coordinate mapping. Keep lossless output explicit and document codec-specific quality settings and client compatibility.

### Testing policy updated on 2026-10-23

The operator cancelled GitHub Actions because of hosted-run costs. Keep Actions disabled and the workflow manual-only until the operator explicitly asks to enable or run it. Use appropriate local checks, including the available Android Studio/emulator, and clearly distinguish local validation from unverified platform checks. This overrides the earlier preference for full PR suites on GitHub.

## Project session correction — 2026-10-23

The user replaced per-action desktop approval with one explicit project Start. Implement [seamless project sessions](specs/seamless-project-sessions.md): independent project Pause/Resume and Stop, a global emergency stop, and synchronized phone/desktop status without a second phone Start. Previously saved directory/device/tool restrictions still apply to every request. This supersedes generic per-action managed approval requirements, including scrolling capture; agent-client sensitive-action approvals remain separate. Keep hosted Actions disabled.

## Preview acceptance decision — 2026-10-24

The operator explicitly chose: “Defer broader accessibility checks for this preview.” Full human TalkBack navigation and broader desktop screen-reader coverage are deferred for the 0.1.2 preview and do not block closing this preview-delivery task. Windows-native testing was already deferred. Existing scoped accessibility and keyboard evidence remains valid; the deferred checks are unverified, not passed. The published preview retains its unsigned-desktop and wider-platform qualifications.

## OpenCode connector — 2026-10-24

Add Connect OpenCode to the desktop app, alongside Codex and Claude. Target the installed OpenCode 2 schema (`mcp.servers`, local command array), preserve existing project JSON/JSONC settings and comments, preview the entry, fence concurrent edits and keep existing project/device/tool authorization. Document the connection and verify it with the installed client.

## Core MCP startup correction — 2026-10-25

Fix the underlying startup/update path rather than repairing individual installations. Stage executable replacements at fresh inodes, ad-hoc sign and verify complete local macOS bundles, verify the installed MCP executable, and retain rollback for failed updates. Connection setup must check the bundled server before writing client configuration. Connect OpenCode must verify the selected project's actual MCP status and recover failed or missing runtime connections using the reviewed entry, without globally reloading unrelated sessions. Preserve directory/device/tool scope and keep hosted Actions disabled.

## Android background reliability — 2026-10-25

Detect battery optimization and Android background restriction state automatically. Add a Background connection settings section and a home-screen link when attention is needed. Request a battery-optimization exemption using Android's owner-approved system flow only when not already exempt; refresh after returning, preserve sessions and provide safe fallbacks. Explain Xiaomi/OEM-specific controls without claiming to change or detect them through unsupported APIs. Report optional battery diagnostics through companion status. Do not interrupt another app with recurring permission/settings prompts or silently change power settings. Keep desktop gateway failures distinct from Android background restrictions.

## Agent tool-selection guidance — 2026-10-25

Make MCP tool selection and required argument shapes clearer. Include executable schema examples for screenshots, taps, swipes, condition waits and fixed batch pauses. Explain native/image coordinate conversion, accessibility limitations in rendered UIs, real conditions versus timers, and correction after validation errors. OpenCode connections should expose complete schemas directly by default (`codemode: false`) while retaining existing project/device/tool scope and client permission policy. Do not claim that descriptions/configuration guarantee model accuracy.
