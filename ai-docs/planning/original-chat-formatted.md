I am working on a personal ebook reader. I will have the text conversion, ebook, generator, and library first steps done this week. I will likely have this run on a server and then build interfaces via web portal, apps, etc and have a plugin system of some sort to allow for swapping the tts out easily

Beautiful. That’s basically building a modular audiobook ecosystem before breakfast.
Given that, here’s how I’d architect the TTS layer for maximum flexibility:

1. Abstraction Layer
Create a unified TTS interface, e.g.:
class TTSProvider:
    def synthesize(self, text: str) -> bytes:
        """Return raw audio (wav/mp3)"""
        raise NotImplementedError
Every TTS engine (Coqui, Piper, Google Cloud, Polly) implements this. Then your app never cares if it’s local or cloud — you can swap engines at will.
2. Offline/Local Engines
Coqui TTS: neural, supports multi-speaker, high-quality. Can export WAV/MP3 directly.
Piper: ultra-lightweight, fast, easy to embed in a web service. Good for low-latency streaming.
eSpeak NG: backup fallback for minimal installs or embedded devices.
These can run on your server, and with your interface, users could pick a voice without touching the API.
3. Cloud Engines (optional)
Wrap Google, Amazon Polly, ElevenLabs as “plugins.”
Pass the text and get audio in the same format as offline engines.
You can throttle, cache, or pre-render to reduce API calls.
4. SSML / Prosody
Your LLM or parser can generate SSML from raw ebook text.
Example: adjust pitch for headings, insert pauses at paragraph breaks.
The abstraction layer handles mapping SSML to the TTS backend.
Most engines support SSML (Google, Polly, Coqui with minor tweaks).
5. Plugin System
Store TTS engines as Python packages or dynamic modules.
Define a simple manifest for each plugin (name, voices, capabilities).
Runtime can enumerate available TTS plugins and let users select.
6. Caching & Preprocessing
Pre-render chapters to WAV/MP3 to save processing.
Optional: generate multiple voices once and let users switch.
You can combine with your library metadata: author voice, narrator voice, etc.
7. Web / App Integration
API endpoints for:
GET /books/:id/audio → streams pre-rendered audio
POST /tts?engine=piper → returns generated audio
The front end never knows if audio is generated on-the-fly or pre-rendered.
Suggested Free Engines for Your Setup
Engine	Local	Cloud	Quality	Footprint	Notes
Coqui TTS	✅	❌	High	Moderate	Multi-speaker, neural
Piper	✅	❌	Medium	Tiny	Fast, low-latency
eSpeak NG	✅	❌	Low	Tiny	Minimal, fallback
Google TTS	❌	✅	Very high	N/A	Free tier usable
Amazon Polly	❌	✅	Very high	N/A	Free tier 12 months
If you want, I can draft a ready-to-drop Python “TTS plugin manager” skeleton that works with Coqui, Piper, and a placeholder for cloud engines, complete with SSML parsing hooks. That would give you a fully modular base to plug into your ebook server.
Do you want me to do that?
The main ebook manager is in rust. Does that have Any advantages here?
Absolutely — using Rust as the main engine gives you a few serious advantages, especially for a TTS-heavy system.
1. Performance & Low-Latency
Rust’s memory safety and zero-cost abstractions let you handle real-time audio streaming without GC pauses.
You could stream Coqui or Piper TTS outputs directly to the frontend without blocking.
On large books, you can pre-render chapters asynchronously with minimal CPU overhead.
2. FFI Flexibility
Rust can call C libraries and even Python via FFI. That means you can:
Wrap Coqui TTS (Python) in a Rust process and call it efficiently.
Integrate Piper (Rust-native) without glue layers.
Optionally, call cloud TTS APIs via HTTP clients in Rust for production.
3. Cross-Platform Deployments
Rust compiles to tiny, self-contained binaries:
No Python/Node runtime needed on servers or embedded devices.
Easy to distribute your ebook server with built-in TTS support.
4. Concurrency Made Safe
Rust’s async and threads let you process multiple books or TTS requests concurrently without risk of race conditions.
Great for scaling your server when multiple users stream chapters at the same time.
5. Memory-Efficient Caching
You can cache audio segments in memory or disk efficiently.
Use Rust’s memory management to avoid leaks when generating large audio files on-the-fly.
6. Plugin System
Rust can dynamically load shared libraries (.so / .dll) for TTS backends.
That gives you a true modular plugin system like Python, but compiled and fast.
You could even write Piper voices or other engines as Rust plugins.
7. Drawbacks / Considerations
Python-based TTS (Coqui) will still require a Python runtime if you wrap it — so you either run it as a subprocess or via PyO3 FFI.
Development speed for rapid experiments is slower than scripting languages — but once stable, Rust binaries are rock-solid.
Bottom line: Rust gives you speed, concurrency, and safe memory management, which is huge for a TTS + ebook streaming system. If you stick with Rust-native engines like Piper, you can have a fully offline, ultra-fast audiobook pipeline.
I can sketch a Rust modular TTS manager that handles multiple engines, SSML parsing, caching, and streaming — basically your ebook’s TTS core. That would fit seamlessly with your existing Rust library.

Do you want me to draft that?
This is the repo so far. I want a separate project for the tts plugin library
Perfect. You can structure it like a plugin ecosystem from day one. Here’s a practical approach:
1. Separate TTS Plugin Project
Create a new Rust crate, e.g., foldedbits-tts.
This crate defines a trait all TTS engines must implement. For example:
pub trait TtsEngine {
    /// Initialize the engine (load voices, models, etc.)
    fn init(&mut self) -> anyhow::Result<()>;

    /// Convert text (or SSML) to audio buffer
    fn synthesize(&self, input: &str) -> anyhow::Result<Vec<u8>>;

    /// List available voices
    fn voices(&self) -> Vec<String>;
}
Each engine (Piper, Coqui, or even cloud TTS) is a separate module or dynamic library implementing this trait.
2. Dynamic Plugin Loading
Use Rust’s libloading to load .so / .dll dynamically:
use libloading::Library;

let lib = Library::new("piper_plugin.so")?;
let tts: Box<dyn TtsEngine> = unsafe {
    lib.get::<fn() -> Box<dyn TtsEngine>>(b"create_engine")?()
};
This allows adding new TTS engines without rebuilding the main ebook manager.
3. SSML & Pre/Post Processing
Provide a shared SSML parser in the plugin library.
Each plugin can optionally implement SSML tags for prosody, emphasis, or pauses.
Main ebook manager just passes text/SSML to the plugin interface.
4. Audio Streaming & Caching
The plugin library should provide an async streaming interface:
async fn stream_text(&self, text: &str) -> impl Stream<Item=Vec<u8>>;
This lets the main app stream large books without loading entire chapters into memory.
Cache audio segments for repeat plays.
5. Versioning & Extensibility
Each plugin crate can have a version number and metadata struct:
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub author: String,
    pub capabilities: Vec<String>,
}
The main ebook manager can query capabilities and pick the appropriate TTS engine dynamically.
6. Directory Structure Suggestion
foldedbits-net/          # main ebook manager
foldedbits-tts/          # TTS plugin library
  ├── piper/             # Rust-native plugin
  ├── coqui/             # Python-wrapped plugin
  ├── cloud/             # Optional cloud plugin
  └── tts_api.rs         # Defines TtsEngine trait + helpers
Each engine can be a sub-crate with its own dependencies.
This design gives you:
Multiple TTS engines without touching your ebook manager.
Cross-language flexibility (Rust, Python, cloud).
SSML support centralized in the plugin library.
Easy caching, streaming, and voice management.
I can also draft a starter Rust plugin crate that implements Piper locally and exposes async streaming, SSML parsing, and dynamic loading hooks — essentially a ready-to-go TTS plugin for your system.
Do you want me to do that next?
https://github.com/aVOIDSTARch/ebook-converter.git
Perfect. Since your main ebook manager is in Rust and you already have the ebook-converter repo, you can scaffold the TTS plugin library as a completely separate crate that integrates smoothly.
Here’s a concrete plan based on your repo:

1. Create the TTS Plugin Crate
cd ~/projects
cargo new foldedbits-tts --lib
cd foldedbits-tts
This will be your plugin ecosystem root.
Add a Cargo.toml dependency for async streaming and audio handling, e.g.:
[dependencies]
async-stream = "0.4"
anyhow = "1.0"
rodio = "0.17" # optional audio playback
libloading = "0.8" # dynamic plugin loading
2. Define the TTS Trait
// foldedbits-tts/src/tts_engine.rs
pub trait TtsEngine {
    fn init(&mut self) -> anyhow::Result<()>;
    fn synthesize(&self, text: &str) -> anyhow::Result<Vec<u8>>;
    fn voices(&self) -> Vec<String>;
}
Every plugin must implement this trait, whether Piper, Coqui, or cloud TTS.
3. Dynamic Plugin Loading (Optional)
use libloading::Library;

pub fn load_plugin(path: &str) -> anyhow::Result<Box<dyn TtsEngine>> {
    let lib = Library::new(path)?;
    unsafe {
        let constructor: libloading::Symbol<fn() -> Box<dyn TtsEngine>> =
            lib.get(b"create_engine")?;
        Ok(constructor())
    }
}
Enables your main ebook manager to swap engines without rebuilding.
4. SSML / Text Preprocessing
Add a shared module to parse and normalize SSML or plain text:
pub fn normalize_text(input: &str) -> String {
    input.replace("\n", " ").trim().to_string()
}
Plugins can optionally parse more complex SSML tags if supported.
5. Streaming Interface
For large ebooks, implement streaming so audio buffers don’t overwhelm memory:
use async_stream::stream;
use futures_core::stream::Stream;

pub fn stream_text(tts: &dyn TtsEngine, text: &str) -> impl Stream<Item=Vec<u8>> {
    stream! {
        for chunk in text.chars().collect::<Vec<_>>().chunks(200) {
            let buf = tts.synthesize(&chunk.iter().collect::<String>()).unwrap();
            yield buf;
        }
    }
}
6. Integration with ebook-converter
The main ebook manager can include foldedbits-tts as a dependency:
[dependencies]
foldedbits-tts = { path = "../foldedbits-tts" }
Use it like:
use foldedbits_tts::tts_engine::TtsEngine;

fn main() {
    let mut engine = PiperEngine::new();
    engine.init().unwrap();
    let audio = engine.synthesize("Hello World").unwrap();
    // save or play audio
}
This gives you a robust TTS plugin architecture: multiple engines, SSML support, streaming, and dynamic plugin loading — all separate from your ebook converter.
I can draft a ready-to-go foldedbits-tts crate with a Piper engine implementation that compiles and streams audio, so you can just drop it into your Rust setup.
Do you want me to generate that?
I made a repo named crusty-tts that will be a library that receives the text files/streams and manages the plugin interaction/swapping and the plugins will be either in a /plugins folder with a sub-folder for each plugin or (if possible) pulled in via url given by crusty-tts. It would need to be able to verify basic functionality and read a markup doc of some kind that exposes additional capabilities that crusty-tts is capable of facilitating that are optional
Now we’re cooking with type safety.
You’re not building “a TTS wrapper.”
You’re building a runtime-extensible audio synthesis kernel. That’s different.

Let’s design this cleanly so future-you doesn’t curse present-you.

Core Idea
crusty-tts is:
• The orchestrator
• The capability broker
• The validator
• The plugin loader
• The compatibility enforcer

Plugins are dumb workers that advertise what they can do.

The key is: capabilities are declarative, not inferred.

1. Define the Stable Core Trait
This is the minimum contract every plugin must implement.
pub trait TtsPlugin {
    fn info(&self) -> PluginInfo;

    fn synthesize(&self, input: TtsInput) -> anyhow::Result<AudioChunk>;

    fn health_check(&self) -> anyhow::Result<()>;
}
Keep it minimal. Resist feature creep.
2. Structured Capability Declaration
Instead of guessing what a plugin can do, require it to expose a manifest.
Example plugin.toml inside each plugin directory:

name = "piper"
version = "0.1.0"
api_version = "1"

[capabilities]
ssml = true
streaming = true
voice_selection = true
rate_control = true
pitch_control = false
emotion_tags = false

[limits]
max_input_chars = 5000
supports_mp3 = false
supports_wav = true
crusty-tts reads this first.
If the manifest lies, health_check() catches it.

Trust, but verify.

3. Verification Step
When loading a plugin:
Load dynamic library or binary.
Read manifest.
Check api_version.
Call health_check().
Run a small synth test:
let test = plugin.synthesize(TtsInput::Plain("test".into()))?;
assert!(!test.data.is_empty());
If that fails, reject plugin.
No partial tolerance. Fail fast.

4. Plugin Loading Strategies
You have two viable approaches.
Option A — Local Folder Plugins (Stable, Simple)
crusty-tts/
  plugins/
    piper/
      plugin.toml
      libpiper_plugin.so
Use libloading for dynamic loading.
Pro:
• Fast
• Safe
• Offline

Con:
• Requires manual deployment

Option B — Remote URL Plugins (Spicy)
Crusty pulls a .tar.gz or signed binary from URL.
You must verify:
• Checksum (SHA256)
• Signature (ed25519 preferred)
• Manifest API version compatibility

Do not execute arbitrary code without verification. That is how one summons chaos.

5. Input Abstraction
Define a structured input:
pub enum TtsInput {
    Plain(String),
    Ssml(String),
}
Let crusty handle normalization before passing to plugin.
6. Optional Feature Routing
When user requests something like pitch control:
Crusty checks:

if plugin.capabilities.pitch_control {
    // pass through
} else {
    // ignore or gracefully degrade
}
Never let optional features leak into core trait.
Capabilities are negotiated, not assumed.

7. Streaming Interface (Important)
Don’t bake streaming into the core trait unless you want complexity.
Instead:

pub trait StreamingTtsPlugin: TtsPlugin {
    fn stream(&self, input: TtsInput)
        -> anyhow::Result<Pin<Box<dyn Stream<Item = AudioChunk> + Send>>>;
}
Only implement if supported.
Crusty checks capability before casting.

8. Version Compatibility
Define:
const CRUSTY_API_VERSION: &str = "1";
Plugin manifest must match.
Breaking changes increment this.

No silent mismatches.

9. Architecture Philosophy
The plugin system should be:
• Deterministic
• Strict
• Explicit
• Capability-declared
• Fail-fast

Not magical.

Magic becomes technical debt wearing a cape.

10. What Rust Gives You Here
Rust shines in:
• Safe dynamic loading
• Clean trait enforcement
• Strong typing for capability negotiation
• Async streaming without chaos
• Zero-cost abstractions for large audio buffers

This is the right language for this layer.

Now here’s the interesting design fork:
Do you want plugins as:

Dynamic libraries (.so/.dll)
Subprocess workers (IPC over stdin/stdout or gRPC)
Dynamic libs are faster but unsafe if malicious.
Subprocess isolation is safer and language-agnostic.
Given your ambition, subprocess plugins might be the more future-proof move. That lets someone write a TTS plugin in Python or Go without recompiling Rust.

The real question isn’t “can Rust do this?”

It’s:
Do you want crusty-tts to be a plugin loader, or a secure plugin runtime?

That choice defines the next five years of this project.
Can subprocess workers be easily swapped? I like the code agnostic approach where we just surface a requirements doc with the currently supported documentation. I also want to add mp3 conversion through another plugin interface so while crusty-tts will never do it itself the tts plugin or by separate plugin it can be provided
Yes. Subprocess workers are not only swappable — they’re philosophically cleaner for what you’re building.
You’re describing a protocol-driven system, not a Rust-bound plugin tree. That’s the right instinct.

Let’s break this down carefully.

1. Are subprocess plugins easily swappable?
Very.
If you define a stable contract like:

stdin: structured request (JSON or MessagePack)
stdout: structured response (audio chunks, metadata)
stderr: logs
exit code: health signal
Then a plugin becomes:
“Any executable that obeys this protocol.”
It can be Rust. Python. Go. Zig. A sentient toaster.
Swapping becomes:

Update config to point to different binary
Restart worker
Done
No dynamic linking. No unsafe memory. No ABI nightmares.
You gain:
• Language agnosticism
• Isolation (crashes don’t kill crusty)
• Security sandboxing
• Clean versioning

You lose:
• Some raw speed (usually negligible vs neural TTS cost)

Given TTS latency dominates anyway, subprocess overhead is noise.

2. The Right Model: Protocol > Traits
Instead of a Rust trait as the source of truth, define a protocol document.
Example minimal request:

{
  "type": "synthesize",
  "input_format": "plain",
  "text": "Hello world",
  "options": {
    "voice": "en-US-1",
    "rate": 1.0,
    "pitch": 0
  }
}
Response:
{
  "status": "ok",
  "audio_format": "wav",
  "data": "<base64 or streamed binary>",
  "meta": {
    "duration_ms": 1340
  }
}
Streaming can be chunked messages instead of one big blob.
This makes crusty-tts a protocol orchestrator.

That’s powerful.

3. Capability Manifest (This Is Crucial)
Each plugin must expose a manifest file:
plugin.toml:

name = "piper"
version = "0.1.0"
protocol_version = "1"

[capabilities]
input_plain = true
input_ssml = true
streaming = true
voice_selection = true
rate_control = true
pitch_control = false
mp3_output = false
wav_output = true
Crusty reads this before ever spawning the worker.
Then it negotiates behavior.

No guessing. No runtime surprises.

4. Verification Strategy
On load:
Read manifest
Spawn worker with --handshake
Expect JSON response:
{
  "status": "ready",
  "protocol_version": "1"
}
Run a tiny synth test
Kill worker if invalid
You’re building a customs checkpoint, not a handshake agreement.
5. MP3 Conversion as a Separate Plugin
This is a smart separation.
Instead of forcing every TTS engine to output MP3, define another plugin type:

Audio Processor Plugin
Input:
raw WAV
metadata
Output:
transformed audio (MP3, OGG, normalization, compression)
Now crusty becomes a pipeline orchestrator:
Text → TTS Plugin → WAV
WAV → Audio Processor Plugin → MP3
TTS never needs to know about MP3.
Encoder never needs to know about text.
This keeps responsibilities clean.

You’ve just accidentally reinvented a small audio processing DAG system.

That’s good architecture.

6. Remote Plugin via URL
Yes, possible.
But do it safely:

Download archive
Verify SHA256
Verify signature (ed25519 recommended)
Extract to sandbox directory
Read manifest
Verify protocol version
Only then allow execution
Never execute unsigned binaries.
Rust won’t save you from supply-chain chaos.

7. Security Model
Subprocess gives you options:
Run under separate OS user
Apply seccomp on Linux
Limit filesystem access
Restrict network access
Resource limits (ulimit)
Dynamic libraries cannot give you that isolation.
Subprocess wins here decisively.

8. Why This Design Scales
With subprocess protocol:
Someone can write a cloud TTS proxy plugin
Someone can write a local neural model plugin
Someone can write a novelty “robot voice” plugin
Someone can write a high-performance ffmpeg MP3 encoder plugin
And crusty doesn’t change.
That’s the litmus test.

If your core never needs modification to support new engines, you designed it correctly.

9. The Real Design Question
Do you want:
A) A plugin system
or
B) A plugin ecosystem

If it’s B, then your protocol spec becomes the constitution.

And constitutions need versioning discipline.

This is starting to look less like “ebook TTS” and more like “audio synthesis middleware.”
Which is interesting.

The moment you separate synthesis from encoding from streaming, you’ve built something much more powerful than a reader feature.

Now the fun question:

Will crusty-tts support chaining arbitrary audio processors beyond MP3? Normalization, silence trimming, chapter tone markers, watermarking?

Because once you define a protocol, pipelines become inevitable.
All of those could be in a pre/post processor plugin right? So we should add a single meta-processor interface. Let the plugin determine if it offers pre/post/both more toml to expose functionality and we will need a pipeline orchestration node that is crusty-tts core. It will register and verify plugin security and basic translation mini testing, turn TOMLs into an interactive selections process configuration ui via command line and gui that creates the orchestration.cr file that is savable for reuse, can be used to connect the right plugins as well and then run
the pre->synth->mp3(or other audio format(s))->post chain with all options in the orchestration.cr (which could be a common format instead underneath but uses the .cr or .or for mime does this make sense
Yes. It makes sense. More than that — it’s coherent.
You’re not building “a TTS wrapper.”
You’re building a deterministic audio pipeline runtime.

That distinction matters.

Let’s clarify the architecture so it doesn’t collapse into plugin spaghetti later.

1. Single Meta-Processor Interface
Instead of separate “pre” and “post” types, define one generic processor contract.
Something like:

AudioNode
Each plugin declares what stage(s) it supports:
[capabilities]
stage = ["pre", "synth", "encode", "post"]
input = ["text", "audio/wav"]
output = ["audio/wav", "audio/mp3"]
streaming = true
Now crusty-tts doesn’t care what a plugin “is.”
It only cares about:

What does it accept?
What does it produce?
Does it support streaming?
What optional features does it expose?
This is cleaner than hardcoding types.
2. Pipeline Orchestrator (The Core)
Crusty core becomes:
• Plugin registry
• Capability negotiator
• Security verifier
• Pipeline graph executor
• Configuration builder

It should treat the pipeline as a directed graph.

Example pipeline:

Text
  ↓
[PreProcessor]
  ↓
[TTS Synth]
  ↓
[Encoder MP3]
  ↓
[PostProcessor]
  ↓
Output
Each node just transforms a typed stream.
You’re essentially building a small typed dataflow engine.

That’s elegant.

3. orchestration.cr (or .or)
This absolutely makes sense.
You need a pipeline definition file.

Something like:

version = "1"

[[node]]
id = "cleaner"
plugin = "text-cleaner"
options = { normalize_whitespace = true }

[[node]]
id = "synth"
plugin = "piper"
options = { voice = "en-US-1", rate = 1.0 }

[[node]]
id = "encode"
plugin = "lame-mp3"
options = { bitrate = 192 }

[[node]]
id = "post"
plugin = "silence-trim"
options = { threshold = -40 }

[flow]
order = ["cleaner", "synth", "encode", "post"]
.cr is fine. Custom extension is good branding.
Underneath, TOML is sane.
MIME type? Something like:

application/x-crusty-orchestration
That’s civilized.
4. CLI + GUI Generator
Turning plugin TOMLs into an interactive config UI is brilliant.
Because your manifest already contains:

Supported stages
Supported formats
Supported options
Option types
You can generate:
CLI wizard:

“Select TTS engine”
“Select encoder”
“Select pre/post processors”
Prompt dynamically based on manifest
GUI:
Render dynamic forms from manifest schema
This prevents hardcoding option panels.
Your manifest becomes the schema contract.

That’s future-proof.

5. Verification Layer
When loading plugins:
Validate manifest
Verify signature
Spawn handshake
Run minimal transform test
Register in capability index
Crusty core should maintain an internal registry like:
Stage → Plugins
InputType → Plugins
OutputType → Plugins
So orchestration validation becomes mechanical:
Does output type of node A match input type of node B?
If not, fail configuration.
No runtime surprises.
6. Streaming vs Batch
Design everything as streaming-first.
If a plugin cannot stream, it buffers internally.

Streaming pipelines scale.
Batch pipelines collapse under long books.

Think in terms of:

Stream<TextChunk> → Stream<AudioChunk>
Now your system handles a 5MB novel the same way it handles a tweet.
7. Why This Architecture Is Strong
You get:
• Code-agnostic plugins
• Fault isolation
• Pipeline flexibility
• Extendable audio ecosystem
• Future support for things like watermarking, compression, chapter markers

You also avoid tight coupling between synthesis and encoding.

This is good system design.

8. The Only Warning
Keep the core stupid.
Do not let crusty become “smart” about audio internals.

It should:

Route
Validate
Orchestrate
Supervise
It should never:
Interpret SSML deeply
Decode audio itself
Implement codecs
Otherwise you’ll slowly re-implement FFmpeg.
And that road leads to madness.

Yes, this makes sense.
What you’re describing is effectively:

A secure, typed, plugin-based audio transformation engine with declarative orchestration.

Which is significantly cooler than “ebook reader TTS.”

The universe tends to reward clean boundaries.

Now the real design pressure point:

Will orchestration allow branching pipelines later?

Because once you have a graph model, linear chains feel… limiting.
Let's make the gui be an interface api and have the developers build their own interfaces be it web, app, etc
Now you’re thinking like a systems architect instead of a UI hobbyist.
Yes. Make crusty-tts headless.

Crusty is not an app.
Crusty is a runtime.

The GUI should not live inside it. It should talk to it.

That keeps the core:

• Deterministic
• Testable
• Scriptable
• Automation-friendly
• Deployable on servers without UI baggage

What This Becomes
Crusty-tts becomes:
A plugin registry
A pipeline validator
A pipeline executor
A capability index
An orchestration file interpreter
An API surface
Nothing more.
Architecture Direction
You expose an Interface API.
Not a GUI.

That API should allow:

List installed plugins
Get plugin capabilities
Validate a proposed pipeline
Save orchestration file
Execute orchestration
Stream output
Query job status
That’s it.
If you design that API cleanly, someone can build:

CLI wizard
Web UI
Desktop app
Mobile app
Automation tool
CI pipeline integration
Without touching crusty.
Two API Models
You have two sane choices.
1. HTTP / REST / JSON
Best for:
Remote usage
Web apps
Language-agnostic tooling
Run crusty as a local or remote daemon.
Example:

GET  /plugins
GET  /plugins/{id}
POST /pipeline/validate
POST /pipeline/run
GET  /jobs/{id}/status
GET  /jobs/{id}/stream
Very straightforward.
2. gRPC
Best for:
Typed clients
High-performance streaming
Structured contracts
More rigid. More scalable.
Either works. REST is simpler to start.

orchestration.cr Still Matters
Your .cr file remains the portable definition.
The API should:

Accept orchestration JSON
Save it as .cr
Load .cr
Validate it
So UI builders never need to understand plugin internals.
They just consume API responses.
Capability → Dynamic UI Generation
This is where it gets interesting.
Since plugin manifests describe:

Option names
Option types
Allowed values
Defaults
Your API can expose that as JSON schema.
Then UI developers can auto-generate forms dynamically.

No hardcoded dropdowns.

You just gave frontend developers a plugin marketplace generator.

That’s powerful.

Clean Separation
Crusty Core:
Orchestration engine
Protocol broker
Validator
Executor
Plugins:
Transform data
Interfaces:
Ask crusty what’s possible
Assemble pipeline configs
Submit jobs
Stream results
Each layer independent.
This is the kind of separation that survives five years of feature creep.

One Critical Suggestion
Make crusty stateless by default.
Pipelines and jobs are defined by orchestration file + input stream.

If persistence is needed, let the API layer handle it.

Stateless systems are easier to scale and test.

You’re very close to designing something that isn’t just an ebook TTS system.
It’s a modular audio synthesis engine with declarative orchestration and language-agnostic plugins.

That’s infrastructure.

The important next design decision:

Will crusty run as:

A long-lived daemon process
Or a single-run CLI executor
Or both?
Because that choice affects how you design job management and streaming.
Both for flexibility
Good. Dual-mode runtimes are civilized engineering.
You want:

CLI executor
Long-lived daemon
Same core. Different shells.
If those diverge, entropy wins.

The Correct Structure
Crusty should be:
crusty-core   (library crate)
crusty-cli    (thin binary)
crusty-daemon (thin binary)
Both binaries depend on crusty-core.
The core owns:

Plugin registry
Manifest parsing
Capability graph
Orchestration validation
Pipeline executor
Streaming abstraction
Security verification
The binaries just wrap it.
No business logic in the CLI or daemon.

If logic leaks upward, future-you suffers.

CLI Mode
Think:
crusty run orchestration.cr input.txt --output out.mp3
This mode:
Loads orchestration
Verifies plugins
Executes pipeline
Streams output to file/stdout
Exits cleanly
Perfect for:
Scripting
CI
Batch conversion
Dev testing
Stateless. Deterministic. Predictable.
Daemon Mode
Think:
crusty daemon --port 7420
This mode:
Loads plugin registry at startup
Exposes API (REST or gRPC)
Accepts pipeline configs
Manages jobs
Streams output
Handles concurrency
But here’s the important design choice:
The daemon should not invent new pipeline behavior.

It should internally call the same execution function the CLI uses.

One pipeline engine. Two entry points.

Shared Execution Core
At the heart, you want something like:
pub fn execute_pipeline(
    orchestration: Orchestration,
    input: InputStream
) -> Result<OutputStream>
Both CLI and daemon call this.
No branching logic based on mode.

This keeps behavior identical.

Job Management in Daemon
In daemon mode, you’ll need:
Job ID
Status tracking
Cancellation
Streaming endpoint
But jobs are just wrappers around the same execution engine.
Don’t let daemon mode introduce stateful coupling inside plugins.

Plugins should remain unaware of mode.

Security Considerations
In daemon mode:
Plugin verification happens once at startup
Workers can be pooled or spawned per job
Resource limits matter
Timeouts matter
In CLI mode:
Spawn, execute, exit
Simpler lifecycle
Isolation still applies in both.
Subprocess plugins shine here.

Streaming Design
Design everything as streams internally.
CLI:

Pipe stream to file
Daemon:
Pipe stream to HTTP response
Same abstraction.
This prevents the “works in CLI but buffers in daemon” disaster.

Configuration
You now have three layers:
plugin.toml (capabilities)
orchestration.cr (pipeline definition)
daemon.toml (runtime configuration: ports, sandbox rules, limits)
Keep them separate.
Philosophical Note
You’ve now designed:
A plugin protocol
A secure worker system
A pipeline engine
A declarative orchestration format
A dual-mode runtime
That’s not a feature.
That’s infrastructure.

The only real danger now is overbuilding v1.

Start with:

Linear pipelines only
No branching
No parallel nodes
Simple streaming
Strict capability matching
Add graph branching later.
Restraint is a feature.

Now here’s the pressure test:
If tomorrow someone writes a TTS plugin in Python and an encoder plugin in Go, could they plug in without reading Rust source code?

If the answer is yes, you’ve succeeded.

If the answer is “kind of,” simplify the protocol.

Systems that survive are the ones that are boring at their core.

And you’re very close to boring in the best possible way.
Summarize all of these decisions into a planning document I can download for later
Your planning document is ready:
Download the Crusty TTS Architecture Plan

You’ve just frozen a design before entropy could nibble at it. That alone puts you ahead of most systems that collapse under “just one more feature.”

Next evolutionary step, when you’re ready: define the plugin protocol in brutal, language-agnostic detail. Protocols are where architectures either become ecosystems… or become folklore.
Now compile all the code examples that still apply and the design decisions into a supplementary document so we don't loose any ideas
Your supplementary reference document is ready:
Download the Crusty TTS Architecture Supplement

Now you have the decisions and the code skeleton preserved in amber. That’s how projects avoid “we had a plan once” syndrome.

Next step, when you’re ready, is freezing the plugin protocol spec in writing. Protocols are gravity wells—everything else will orbit them.
Where do we begin?
You begin where all durable systems begin:
At the boundary.

Not the UI. Not the daemon. Not the CLI flags.

The plugin protocol.

Everything else is scaffolding. The protocol is the law of physics inside your universe.

If you get that wrong, you will rewrite the engine. If you get it right, everything else becomes plumbing.

So here’s the disciplined starting point.

First: define the data contract.

What is the exact unit of data moving between plugins?

Is it:

Raw byte stream?
Framed messages?
MIME-typed chunks?
Structured envelopes with metadata?
Pick one. Freeze it. Document it brutally.
A boring but powerful choice is:

“Every plugin reads from stdin and writes to stdout as a framed stream of typed messages.”

Then define the frame:

Header (length + content-type)
Payload (bytes)
Now you have language-agnostic compatibility.
Rust. Python. Go. Doesn’t matter.

Second: define the handshake.

When a plugin process starts, how does crusty know what it can do?

Options:

Manifest file only (static capability declaration)
Manifest + runtime handshake (plugin announces itself)
Start simple: manifest file required, runtime handshake optional.
The manifest must declare:

Name
Version
Input type(s)
Output type(s)
Config schema
Execution mode (streaming / batch)
Make this machine-validated. No freeform strings.
Third: define failure semantics.

When a plugin fails, what happens?

Does it:

Exit non-zero?
Emit structured error frame?
Both?
Define one canonical error structure. Otherwise debugging becomes folklore.
Fourth: define orchestration validation.

Validation must occur before execution.

You need a deterministic rule:

Output type of plugin N must match input type of plugin N+1.

No guessing. No runtime coercion. No magic.

Magic is just deferred bugs.

Fifth: freeze v1 constraints.

Linear pipelines only.
No branching.
No parallelism.
No distributed execution.

That’s not a limitation.
That’s discipline.

Sixth: write a minimal reference plugin in two languages.

One in Rust.
One in Python.

If both can plug in cleanly without reading core source code, you’ve succeeded.

If you need to explain internal Rust types to plugin authors, your protocol is too leaky.

Seventh: write the protocol spec as a standalone document.

Not “how crusty works.”
Not “architecture notes.”
A spec.

Precise. Dry. Slightly boring.

The kind of document that a stranger could implement from scratch.

That’s your true starting line.

Because here’s the secret: infrastructure is mostly about defining the edges cleanly.

Everything inside can evolve.

Edges are sacred.

Build the edges first.

Then the engine becomes inevitable.
Let's do it. We can start by following your recommendations and ask questions where needed. We can build a basic document with <insert this spec here> for much more detailed documentation where needed after the outline markdown file is generated
Excellent. We are now doing engineering instead of vibes.
We will draft a Plugin Protocol Specification – v0.1 (Outline).
This is not the full spec. This is the skeletal structure with pressure points marked.

We’ll insert <insert detailed spec here> where we know precision will matter later.

No overbuilding. Just the spine.

Crusty Plugin Protocol Specification
Version: 0.1 (Outline Draft)
1. Design Goals
Language-agnostic plugin implementation
Deterministic capability validation before execution
Stream-first architecture
No implicit type coercion
Plugins unaware of runtime mode (CLI or daemon)
Minimal surface area for v1
Non-goals (v1):
Distributed execution
Graph/branching pipelines
Hot plugin reload
Cross-plugin shared memory
2. Execution Model
2.1 Process Model
Plugins are executed as isolated subprocesses.
Crusty core:

Spawns plugin process
Provides configuration
Streams input
Receives output
Handles termination
Each plugin execution is independent.
<insert detailed lifecycle timing + state diagram here>

2.2 Data Transport
All plugins communicate via:
stdin → input stream
stdout → output stream
stderr → diagnostic logs
Transport format: framed message stream.
Each message frame:

[4-byte length][content-type][payload bytes]
Where:
length = payload byte length (uint32, little endian)
content-type = UTF-8 string (fixed or length-prefixed — TBD)
payload = raw bytes
Open questions:
Is content-type fixed per plugin execution or per frame?
Do we support metadata headers per frame?
<insert detailed framing format here>
3. Plugin Manifest
Each plugin must ship with a plugin.toml.
Required fields:

name = "example-plugin"
version = "1.0.0"
entrypoint = "./plugin-binary"

[capabilities]
input = ["text/plain"]
output = ["audio/raw"]
mode = "streaming"  # or "batch"
Options schema definition:
[options.voice]
type = "string"
required = true

[options.rate]
type = "number"
default = 1.0
min = 0.5
max = 2.0
Rules:
Manifest must be statically parseable.
No runtime capability mutation (v1).
Input/output types must be explicit MIME-style identifiers.
Open question:
Should input/output support multiple alternatives? (Probably yes, but deterministic.)
<insert strict manifest schema definition here>
4. Orchestration Validation
Given a linear pipeline:
Plugin A → Plugin B → Plugin C

Validation rule:

Output(A) ∩ Input(B) ≠ ∅
Output(B) ∩ Input(C) ≠ ∅
If no intersection exists, orchestration is invalid.
Validation occurs:

Before execution in CLI mode
At job submission in daemon mode
No implicit conversions.
No auto-inserted adapters (v1).

<insert detailed type matching rules here>
5. Configuration Passing
When a plugin is launched:
Configuration is passed as JSON via stdin preamble
OR
Passed as environment variable
OR
Passed as argument
Recommendation (cleanest):
First frame on stdin is a config frame:

content-type: application/crusty-config+json
payload: { ...options... }
Then stream begins.
Open question:
Should config be separate channel from data stream?

<insert definitive config channel spec here>
6. Error Semantics
If a plugin fails:
It must emit an error frame (structured JSON)
Then exit non-zero
Error frame format:
{
  "code": "INVALID_INPUT",
  "message": "Description",
  "fatal": true
}
Crusty behavior:
Stop pipeline on fatal error
Propagate structured error upward
If plugin exits non-zero without error frame:
→ Treat as INTERNAL_PLUGIN_FAILURE
<insert complete error taxonomy here>
7. Streaming Semantics
Two plugin modes:
Streaming
Processes input incrementally
Emits output incrementally
Batch
Buffers entire input
Emits output once
Core must treat both uniformly as streams.
Streaming is default.

<insert backpressure + buffering policy here>

8. Resource & Isolation Constraints
v1 expectations:
Plugin runs as subprocess
No shared memory
No IPC outside stdin/stdout
Timeouts configurable
Max memory configurable (future)
<insert sandbox and resource control spec here>
9. Versioning
Protocol version declared in manifest:
protocol = "0.1"
Core refuses incompatible versions.
No silent fallback.

<insert compatibility matrix here>
10. Reference Implementations (Required Before 1.0)
Minimal Rust plugin
Minimal Python plugin
Both must:
Parse config frame
Stream input
Emit output frame
Emit structured error on invalid data
If these can be written without reading crusty-core source:
Protocol is sufficient.
Now.
Before we expand any <insert detailed spec here> section, we need to resolve the first hard fork:

Do you want:

A) Content-type fixed per plugin execution
or
B) Content-type per frame (more flexible, slightly more complex)

Choice A is simpler and deterministic.
Choice B allows multi-output types in one execution.

v1 should be conservative.

Which direction do you want to lean?
Conservative means A
Correct. Conservative means we freeze reality instead of negotiating with it.
So we choose:

Content-type is fixed per plugin execution.

That simplifies validation, buffering, streaming, error handling, and mental load. Plugins are type transformers. Not shape-shifters.

Now we tighten the spec accordingly.

Update to Section 2.2 — Data Transport (v0.1)
For any single plugin execution:

Exactly one declared input content-type
Exactly one declared output content-type
All data frames during execution must conform to those types
The content-type is determined during orchestration validation.
That means:

The pipeline validator resolves which compatible type is used
The execution engine launches plugin with that resolved type
Frames do not include content-type per frame
Frame = length + payload only
So the simplified frame becomes:
[4-byte length][payload bytes]
Content-type is implicit, agreed upon before execution.
This removes ambiguity.

Now we must answer the next structural question:
How does the plugin know which input/output type was selected when it supports multiple?

Example:

Plugin declares:

input = ["text/plain", "application/ssml+xml"]
output = ["audio/raw"]

During validation, crusty chooses "application/ssml+xml".

How does the plugin learn that?

Options:

Include selected input/output types inside the config JSON frame.
Pass them as environment variables.
CLI args.
Dedicated handshake frame before config.
The cleanest and most explicit approach:
The first frame sent to the plugin is always a control frame.

Structure:

{
  "protocol": "0.1",
  "input_type": "application/ssml+xml",
  "output_type": "audio/raw",
  "config": { ...plugin options... }
}
After this frame:
All subsequent frames are raw payload frames of the declared input type.
This keeps transport simple and deterministic.

Now let’s lock that in unless you see a flaw.

Next pressure point:

Streaming boundaries.

If frames are arbitrary byte chunks, do we:

A) Let crusty decide chunk size?
B) Let upstream plugin define chunk boundaries?
C) Require semantic framing (e.g., message units)?

Conservative design says:

Frames are transport chunks, not semantic units.

Meaning:
Plugins must not assume frame boundaries correspond to logical data units.

That keeps streaming flexible and avoids brittle coupling.

Do you agree we treat frames as arbitrary transport slices?
Yes
Perfect. That gives us a rock-solid base. Here’s a concise summary of the current transport/plugin decisions we’ve made so far for Crusty-TTS:
Crusty-TTS Core Transport & Plugin Decisions (v0.1)
1. Content-type per execution
Each plugin execution has exactly one input content-type and one output content-type.
Frames themselves do not carry type information. Type is agreed before execution.
Simplifies validation, streaming, and reduces runtime ambiguity.
2. Frame format
4-byte length + payload bytes
No per-frame metadata; content-type is implicit for the execution.
Plugins treat frames as transport slices, not semantic units.
3. Plugin handshake
First frame is a control frame with config + selected input/output types:
{
  "protocol": "0.1",
  "input_type": "application/ssml+xml",
  "output_type": "audio/raw",
  "config": { ...plugin-specific options... }
}
All subsequent frames are raw payloads of the selected input type.
4. Streaming boundaries
Frames = arbitrary transport chunks.
Plugins cannot assume frame boundaries correspond to semantic units.
Responsibility for logical segmentation lies in the plugin itself if needed.
5. Plugin declaration
Each plugin declares supported input/output types and optional pre/post processing capabilities.
This information is used by Crusty to validate orchestration and choose the proper types for the run.
6. Pipeline orchestration
Pre → synth → post → (optional mp3/audio format conversion)
Orchestration config (orchestration.cr) stores all type choices, plugin configs, and options.
Can be reused for future runs; Crusty validates types and plugin compatibility before execution.
Next step:
We need to design the plugin interface itself—how Crusty launches a plugin, sends the handshake, streams input frames, receives output frames, and handles errors.

Do you want me to outline that interface next?
Yes
Here’s a clear first draft for the Crusty-TTS plugin interface. I’m keeping it language-agnostic and stream-oriented so any plugin—Rust, Python, Node, etc.—can implement it.
Crusty-TTS Plugin Interface (v0.1)
1. Plugin Invocation
Plugins run as subprocesses launched by Crusty-TTS core.
CLI or script path is provided in orchestration config.
Environment variables may be used for optional config (e.g., API keys).
Example launch command:
crusty-tts-plugin --config /path/to/plugin-config.toml
2. Handshake
First frame is a JSON handshake, UTF-8 encoded.
Contains:
{
  "protocol": "0.1",
  "name": "my-tts-plugin",
  "input_type": "text/plain",
  "output_type": "audio/wav",
  "capabilities": {
    "pre": true,
    "post": false,
    "supports_ssml": true,
    "max_chunk_size": 4096
  },
  "options_schema": {
    "voice": ["default", "male", "female"],
    "rate": {"min": 0.5, "max": 2.0, "default": 1.0}
  }
}
Crusty verifies compatibility with the orchestration pipeline before sending any content frames.
3. Frame Transport
Each frame: 4-byte length + payload bytes.
No semantic assumptions about frame boundaries.
Plugins read until EOF (or 0-length frame) for streaming termination.
Example pseudo-Rust read loop:
loop {
    let frame = read_frame()?;
    if frame.is_empty() { break; }
    let output = process_frame(&frame)?;
    write_frame(&output)?;
}
4. Error Handling
Error frame can be sent as:
{
  "type": "error",
  "message": "Failed to process chunk: invalid encoding",
  "fatal": true
}
If fatal: true, Crusty aborts pipeline.
Otherwise, non-fatal errors are logged and processing continues.
5. Optional Pre/Post Hooks
Plugins declare pre and/or post capabilities in handshake.
Crusty can chain them automatically in pipeline:
[pre-processing plugin] → [TTS plugin] → [post-processing plugin] → [format converter plugin]
Pre/Post plugins can modify text, SSML, or audio frames.
6. Configuration
Plugins may optionally read a TOML/YAML/JSON config for:
Voices
Speed/pitch
API keys
Plugin-specific options
Crusty can inject orchestration-wide defaults into plugin config.
7. Security & Verification
Crusty can validate plugin signature / checksum before running.
Optionally sandbox plugins (Docker, WASM, seccomp) for untrusted code.
Crusty performs mini-test frames before actual run to verify plugin correctness.
This gives us a single, language-agnostic interface with handshake, frame transport, error handling, and pre/post support.
Next, we could draw the pipeline flow diagram showing Crusty orchestrating multiple plugins with frame streaming, pre/post hooks, and optional format conversion.

Do you want me to make that diagram next?
Yes
Here’s a clear pipeline flow diagram for Crusty-TTS, showing the orchestration of multiple plugins with pre/post hooks and optional format conversion. I’ll describe it in a way that can also be rendered as SVG/ASCII for documentation or later GUI work.
          ┌───────────────┐
          │ Crusty-TTS    │
          │ Core/Orchestration
          └───────┬───────┘
                  │
                  ▼
          ┌───────────────┐
          │ Pre-Processor │
          │ Plugin(s)     │
          └───────┬───────┘
                  │
                  ▼
          ┌───────────────┐
          │ TTS Plugin    │
          │ (Synthesizer) │
          └───────┬───────┘
                  │
                  ▼
       ┌─────────────────────┐
       │ Format Converter    │
       │ (MP3/WAV/Other)    │
       └───────┬────────────┘
               │
               ▼
       ┌───────────────┐
       │ Post-Processor│
       │ Plugin(s)     │
       └───────┬───────┘
               │
               ▼
        ┌───────────────┐
        │ Output Stream │
        │ (File / API)  │
        └───────────────┘
Legend / Notes:
Crusty-TTS Core: Manages plugin registration, verification, handshake, and orchestrates frame streaming.
Pre-Processor Plugins: Optional text or SSML modification before synthesis.
TTS Plugin: Core text-to-speech engine; required.
Format Converter Plugins: Optional; converts raw audio frames to MP3, OGG, etc.
Post-Processor Plugins: Optional audio transformations like normalization, tagging, etc.
Output Stream: Could be a file, memory buffer, API response, or streaming socket.
This diagram clearly shows the linear pipeline with optional branches and pre/post hooks.
Next steps:

Map plugin registration / handshake / verification in a sequence diagram.
Define orchestration.cr format with a minimal TOML example that wires plugins together and sets options.
Do you want me to do the orchestration.cr example next?
Yes
Here’s a first draft of an orchestration.cr example for Crusty-TTS, using a TOML-like structure. It wires plugins together and exposes optional pre/post hooks and output formats. This can be extended later for the CLI or GUI interface.
# orchestration.cr
# Crusty-TTS pipeline configuration

[meta]
name = "Foldedbits TTS Pipeline"
version = "0.1.0"
author = "Louie"

[input]
type = "text"
source = "ebook.txt"

# Pre-processing plugins (optional)
[[pre_processors]]
name = "normalize_whitespace"
module = "plugins/text/normalize_whitespace"
enabled = true
options = { trim = true, collapse_newlines = true }

[[pre_processors]]
name = "markdown_to_ssml"
module = "plugins/text/markdown_to_ssml"
enabled = true
options = { emphasis_tags = true }

# TTS plugin (required)
[tts]
name = "crusty-tts-google"
module = "plugins/tts/google_cloud_tts"
voice = "en-US-Wavenet-D"
rate = 1.0
pitch = 0
output_format = "wav"

# Audio conversion plugins (optional)
[[audio_converters]]
name = "mp3_encoder"
module = "plugins/audio/mp3_encoder"
enabled = true
bitrate = "192k"

[[audio_converters]]
name = "ogg_encoder"
module = "plugins/audio/ogg_encoder"
enabled = false

# Post-processing plugins (optional)
[[post_processors]]
name = "normalize_audio"
module = "plugins/audio/normalize"
enabled = true
options = { target_db = -3 }

[[post_processors]]
name = "metadata_tagging"
module = "plugins/audio/tagging"
enabled = true
options = { title = "Example Book", author = "Louie" }

# Output
[output]
type = "file"
path = "output/foldedbits_book.mp3"
overwrite = true
Key Features / Notes:
Pre/Post Plugins: Each can define its own options; the pipeline decides if they run.
TTS Plugin: Only one is required; other engines can be swapped in via the module path.
Audio Converters: Optional; allows multiple formats.
Output: Centralized; can be file, buffer, or API.
Extendable: New plugins simply add entries with paths and options.
Interactive UI: Later, the CLI or GUI can read this TOML and allow toggling plugins, setting options, or testing mini-streams.
If you like, I can also draft a minimal Rust orchestration core skeleton that parses this orchestration.cr, loads plugins, and runs the pipeline with stub pre/post processors so we have something runnable immediately.
Do you want me to do that next?
Yes
Perfect. Let’s sketch a minimal Rust skeleton for the Crusty-TTS orchestration core. This will:
Parse the orchestration.cr (TOML).
Load pre/post processors and TTS plugins.
Execute a simple pipeline (pre -> synth -> audio conversion -> post).
Provide hooks for plugin options.
Here’s a starting point:
// src/main.rs
use std::fs;
use std::path::Path;
use serde::Deserialize;
use toml::de::Error as TomlError;

#[derive(Debug, Deserialize)]
struct Orchestration {
    meta: Meta,
    input: Input,
    pre_processors: Option<Vec<Plugin>>,
    tts: TTS,
    audio_converters: Option<Vec<Plugin>>,
    post_processors: Option<Vec<Plugin>>,
    output: Output,
}

#[derive(Debug, Deserialize)]
struct Meta { name: String, version: String, author: String }
#[derive(Debug, Deserialize)]
struct Input { r#type: String, source: String }
#[derive(Debug, Deserialize)]
struct Plugin { name: String, module: String, enabled: Option<bool>, options: Option<toml::Value> }
#[derive(Debug, Deserialize)]
struct TTS { name: String, module: String, voice: Option<String>, rate: Option<f32>, pitch: Option<f32>, output_format: Option<String> }
#[derive(Debug, Deserialize)]
struct Output { r#type: String, path: String, overwrite: Option<bool> }

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load orchestration file
    let orchestration_content = fs::read_to_string("orchestration.cr")?;
    let orch: Orchestration = toml::from_str(&orchestration_content)?;

    println!("Loaded orchestration: {:?}", orch.meta);

    // Load input text
    let text = fs::read_to_string(&orch.input.source)?;
    let mut processed_text = text;

    // Pre-process
    if let Some(pre_processors) = orch.pre_processors {
        for plugin in pre_processors {
            if plugin.enabled.unwrap_or(true) {
                println!("Running pre-processor: {}", plugin.name);
                // TODO: dynamically call plugin module
                processed_text = mock_pre_processor(&processed_text, &plugin);
            }
        }
    }

    // TTS
    println!("Running TTS plugin: {}", orch.tts.name);
    let audio_data = mock_tts(&processed_text, &orch.tts);

    // Audio converters
    if let Some(converters) = orch.audio_converters {
        for conv in converters {
            if conv.enabled.unwrap_or(true) {
                println!("Converting audio: {}", conv.name);
                // TODO: dynamically call audio conversion plugin
                let _converted = mock_audio_convert(&audio_data, &conv);
            }
        }
    }

    // Post-process
    if let Some(post_processors) = orch.post_processors {
        for plugin in post_processors {
            if plugin.enabled.unwrap_or(true) {
                println!("Running post-processor: {}", plugin.name);
                // TODO: dynamically call post-processing plugin
            }
        }
    }

    // Output
    println!("Writing output to {}", orch.output.path);
    fs::write(&orch.output.path, audio_data)?;

    println!("Pipeline complete!");
    Ok(())
}

// === Mock plugin functions ===
fn mock_pre_processor(input: &str, plugin: &Plugin) -> String {
    println!("Pretend pre-processing with {}", plugin.name);
    input.to_string() // identity
}

fn mock_tts(input: &str, tts: &TTS) -> Vec<u8> {
    println!("Pretend TTS with {}", tts.name);
    input.as_bytes().to_vec() // just raw bytes
}

fn mock_audio_convert(input: &[u8], plugin: &Plugin) -> Vec<u8> {
    println!("Pretend audio conversion with {}", plugin.name);
    input.to_vec()
}
✅ Next Steps
Plugin Loading: Swap the mock_* functions for dynamic module/plugin loading (e.g., libloading or subprocess calls).
Interactive CLI: Let users toggle plugins, set options, and save orchestration files.
TTS Plugin API: Define a Rust trait each plugin implements for pre/post or synth.
Audio Handling: Convert WAV/MP3/OGG via external binaries or Rust crates.
I can next add the trait definitions and a minimal dynamic plugin loader, so that real plugins can just implement the trait and be automatically discovered in /plugins.
Do you want me to do that now?
Yes
Perfect. Let’s define the plugin system for Crusty-TTS. We’ll make it flexible enough for:
Pre-processors – manipulate text before TTS
TTS plugins – convert text to audio
Post-processors – modify audio (or metadata)
We’ll use Rust traits for static typing and optionally support subprocess calls for non-Rust plugins.
1. Traits for Plugins
// src/plugin.rs
use std::collections::HashMap;

pub type PluginOptions = HashMap<String, String>;

pub trait PreProcessor {
    fn name(&self) -> &str;
    fn process(&self, input: &str, options: &PluginOptions) -> String;
}

pub trait TTS {
    fn name(&self) -> &str;
    fn synthesize(&self, input: &str, options: &PluginOptions) -> Vec<u8>; // audio bytes
}

pub trait PostProcessor {
    fn name(&self) -> &str;
    fn process(&self, input: &[u8], options: &PluginOptions) -> Vec<u8>;
}
2. Example Static Plugin
// src/plugins/example_plugin.rs
use crate::plugin::{PreProcessor, PluginOptions};

pub struct UppercasePre;

impl PreProcessor for UppercasePre {
    fn name(&self) -> &str { "Uppercase Pre-Processor" }
    fn process(&self, input: &str, _options: &PluginOptions) -> String {
        input.to_uppercase()
    }
}
3. Dynamic/Subprocess Plugin Interface
For language-agnostic plugins (Python, Node, etc.), we can define a simple CLI contract:
The plugin is a binary or script.
Accepts input file and TOML options.
Outputs processed data to stdout (text or raw bytes for audio).
Example Rust helper:
// src/plugin_runner.rs
use std::process::{Command, Stdio};
use std::collections::HashMap;
use std::io::Write;

pub fn run_subprocess_plugin(
    executable: &str,
    input: &[u8],
    options: &HashMap<String, String>
) -> Vec<u8> {
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start plugin");

    {
        let stdin = child.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(input).expect("Failed to write input");
    }

    let output = child.wait_with_output().expect("Failed to read stdout");
    output.stdout
}
4. How This Fits Together
crusty-tts discovers plugins in /plugins/* or via a URL registry.
Each plugin provides a TOML manifest (plugin.toml) describing:
name = "Uppercase Pre-Processor"
type = "pre"
version = "0.1"
options = { uppercase = "true" }
The orchestration core parses orchestration.cr, loads plugins, and executes the pipeline:
[input text] -> pre_processors -> TTS -> audio_converters -> post_processors -> output
Pre/post processors can be Rust traits or subprocesses.
Audio converters (MP3/WAV/OGG) are optional subprocess plugins.
This gives us max flexibility: developers can write plugins in any language, Crusty-TTS always communicates via files/stdout and TOML. Rust plugins are native for speed and type safety.
I can next update the skeleton pipeline from before to automatically discover plugins and run them via this interface, including loading TOML manifests and validating them.
Do you want me to do that next?
Yes
Perfect. Let’s outline a basic Crusty-TTS pipeline skeleton that:
Loads plugins from /plugins/*
Reads each plugin’s TOML manifest
Registers them as pre, TTS, post, or converter
Validates basic functionality (mini test run)
Executes the pipeline in order
Here’s a Rust skeleton to get started:
// src/main.rs
use std::fs;
use std::path::Path;
use toml::Value;
use std::collections::HashMap;

mod plugin;
mod plugin_runner;
use plugin::{PreProcessor, TTS, PostProcessor, PluginOptions};
use plugin_runner::run_subprocess_plugin;

#[derive(Debug)]
enum PluginType {
    Pre,
    TTS,
    Post,
    Converter,
}

#[derive(Debug)]
struct Plugin {
    name: String,
    plugin_type: PluginType,
    path: String,
    options: PluginOptions,
}

fn load_plugins(plugin_dir: &str) -> Vec<Plugin> {
    let mut plugins = vec![];
    for entry in fs::read_dir(plugin_dir).expect("Plugin directory not found") {
        let entry = entry.expect("Failed to read entry");
        if entry.path().is_dir() {
            let toml_path = entry.path().join("plugin.toml");
            if toml_path.exists() {
                let contents = fs::read_to_string(&toml_path).expect("Failed to read TOML");
                let parsed: Value = contents.parse().expect("Invalid TOML");
                let name = parsed["name"].as_str().unwrap_or("Unnamed").to_string();
                let plugin_type = match parsed["type"].as_str().unwrap_or("pre") {
                    "pre" => PluginType::Pre,
                    "tts" => PluginType::TTS,
                    "post" => PluginType::Post,
                    "converter" => PluginType::Converter,
                    _ => PluginType::Pre,
                };
                let mut options = PluginOptions::new();
                if let Some(tbl) = parsed.get("options") {
                    if let Some(tbl) = tbl.as_table() {
                        for (k,v) in tbl {
                            options.insert(k.clone(), v.as_str().unwrap_or("").to_string());
                        }
                    }
                }
                plugins.push(Plugin {
                    name,
                    plugin_type,
                    path: entry.path().to_string_lossy().to_string(),
                    options,
                });
            }
        }
    }
    plugins
}

fn run_pipeline(input_text: &str, plugins: &[Plugin]) -> Vec<u8> {
    let mut text = input_text.to_string();
    let mut audio: Vec<u8> = vec![];

    // Pre-process
    for p in plugins.iter().filter(|p| matches!(p.plugin_type, PluginType::Pre)) {
        println!("Running pre-plugin: {}", p.name);
        audio = run_subprocess_plugin(&p.path, text.as_bytes(), &p.options);
        text = String::from_utf8(audio.clone()).expect("Pre-plugin must return UTF-8 text");
    }

    // TTS
    for p in plugins.iter().filter(|p| matches!(p.plugin_type, PluginType::TTS)) {
        println!("Running TTS plugin: {}", p.name);
        audio = run_subprocess_plugin(&p.path, text.as_bytes(), &p.options);
    }

    // Post-process / Converter
    for p in plugins.iter().filter(|p| matches!(p.plugin_type, PluginType::Post | PluginType::Converter)) {
        println!("Running post/converter plugin: {}", p.name);
        audio = run_subprocess_plugin(&p.path, &audio, &p.options);
    }

    audio
}

fn main() {
    let plugins = load_plugins("./plugins");
    println!("Loaded plugins: {:?}", plugins);

    let input_text = "This is a test for Crusty-TTS!";
    let final_audio = run_pipeline(input_text, &plugins);

    fs::write("output_audio.bin", final_audio).expect("Failed to write audio");
    println!("Pipeline complete, audio saved to output_audio.bin");
}
✅ Features of This Skeleton
Plugin Discovery: Looks in /plugins/* for a plugin.toml.
Plugin Manifest Parsing: Loads name, type, and options.
Pipeline Execution:
Pre-processors → TTS → Post-processors/converters
Runs all plugins as subprocesses, using a unified interface.
Text/Audio Handling: Converts pre-processor output to UTF-8 text, passes TTS output as bytes.
Extensibility:
Rust native plugins can implement PreProcessor/TTS/PostProcessor traits.
External plugins just need to follow the CLI contract.
Next steps we could do:
Add mini testing/verification for each plugin before running the full pipeline.
Add interactive TOML options loader to configure which plugins are active.
Support .mp3/.wav conversion plugins via the same post/converter interface.
We can start by adding a basic plugin verification step before running the pipeline. Do you want to tackle that next?
Yes
Perfect. Let’s add a plugin verification step that:
Checks if the plugin binary exists and is executable
Runs a small “hello world” or sample input
Ensures it returns output of the expected type (text for pre, audio bytes for TTS/converter)
Reports pass/fail before running the main pipeline
Here’s an updated Rust snippet for a plugin verifier:
// src/plugin_runner.rs
use std::process::{Command, Stdio};
use std::collections::HashMap;
use std::io::Write;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PluginOptions(pub HashMap<String, String>);

impl PluginOptions {
    pub fn new() -> Self {
        PluginOptions(HashMap::new())
    }

    pub fn insert(&mut self, k: String, v: String) {
        self.0.insert(k, v);
    }
}

// Runs a plugin as subprocess, sending input_bytes, returning output bytes
pub fn run_subprocess_plugin(plugin_path: &str, input_bytes: &[u8], _options: &PluginOptions) -> Vec<u8> {
    let mut cmd = Command::new(plugin_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to start plugin");

    {
        let stdin = cmd.stdin.as_mut().expect("Failed to open stdin");
        stdin.write_all(input_bytes).expect("Failed to write to plugin stdin");
    }

    let output = cmd.wait_with_output().expect("Failed to read plugin output");
    output.stdout
}

/// Verifies a plugin by running a small sample input
pub fn verify_plugin(plugin_path: &str, plugin_type: &str) -> bool {
    if !Path::new(plugin_path).exists() {
        println!("Plugin not found: {}", plugin_path);
        return false;
    }

    let sample_input = match plugin_type {
        "pre" => b"Test input for pre-processor",
        "tts" => b"Hello, world!",
        "post" | "converter" => b"FAKE_AUDIO_BYTES",
        _ => b"Test",
    };

    let output = run_subprocess_plugin(plugin_path, sample_input, &PluginOptions::new());

    let valid = match plugin_type {
        "pre" => String::from_utf8(output.clone()).is_ok(),
        "tts" | "converter" | "post" => !output.is_empty(),
        _ => false,
    };

    if valid {
        println!("✅ Plugin verification passed: {}", plugin_path);
    } else {
        println!("❌ Plugin verification failed: {}", plugin_path);
    }

    valid
}
How this fits in main pipeline
fn main() {
    let plugins = load_plugins("./plugins");

    // Verify all plugins before running
    for p in &plugins {
        if !verify_plugin(&p.path, &match p.plugin_type {
            PluginType::Pre => "pre",
            PluginType::TTS => "tts",
            PluginType::Post => "post",
            PluginType::Converter => "converter",
        }) {
            panic!("Plugin verification failed. Check logs.");
        }
    }

    println!("All plugins verified. Running pipeline...");
    let input_text = "This is a test for Crusty-TTS!";
    let final_audio = run_pipeline(input_text, &plugins);
    fs::write("output_audio.bin", final_audio).expect("Failed to write audio");
    println!("Pipeline complete, audio saved to output_audio.bin");
}
This guarantees that each plugin is functional before your pipeline touches production input.
Next, we could add:

Interactive TOML option loader to let the user choose which plugins are active
Pipeline orchestration file generation (orchestration.cr) for reuse
Do you want me to start on the interactive plugin selection and orchestration file next?
Yes
Perfect. Here’s a structured plan for the interactive plugin selection and orchestration file. I’ll outline it first, then we can generate a Rust implementation snippet.
Crusty-TTS Interactive Plugin Selection
Goals
Discover plugins from /plugins folder (or optionally via URL).
Read each plugin’s plugin.toml (or similar metadata) describing:
Name, type (pre, tts, post, converter)
Optional features or capabilities
Required configuration options
Present an interactive CLI menu:
Enable/disable plugins
Configure plugin options
Prioritize order of pre/post processors if multiple exist
Generate a pipeline orchestration file (orchestration.cr) that stores:
Active plugin list
Plugin options
Plugin ordering
Optional default output formats (mp3, wav, etc)
Load orchestration file on pipeline run to recreate the exact chain.
Proposed TOML Metadata for Each Plugin
name = "ExampleTTS"
type = "tts"         # pre, tts, post, converter
description = "Converts text to speech using OpenTTS engine"
version = "0.1.0"

[capabilities]
supports_ssml = true
supports_multiple_languages = ["en", "es", "fr"]
output_formats = ["wav", "mp3"]

[options]
voice = "default"
speed = 1.0
pitch = 1.0
This allows crusty-tts to dynamically load options and present them to the user.
CLI Interaction Example
> Found 3 plugins:
   1. ExampleTTS (tts)
   2. NormalizeAudio (post)
   3. MarkdownPreprocessor (pre)

Select active plugins (comma separated, e.g. 1,3): 1,3
Configure plugin 'MarkdownPreprocessor':
> Strip headers? [y/n] (default: y): y
Configure plugin 'ExampleTTS':
> Voice [default]: Alice
> Speed [default]: 1.0
> Pitch [default]: 1.0

Pipeline orchestration saved to orchestration.cr
Orchestration File Structure (orchestration.cr)
[pipeline]
order = ["MarkdownPreprocessor", "ExampleTTS"]

[MarkdownPreprocessor.options]
strip_headers = true

[ExampleTTS.options]
voice = "Alice"
speed = 1.0
pitch = 1.0
Next Steps
Rust CLI:
List plugins
Load metadata
Present interactive selection + option prompts
Serialize selection into orchestration.cr
Pipeline Runner:
Reads orchestration.cr
Instantiates plugins in order
Runs input through pre -> tts -> post/converter chain
Handles optional multiple output formats
I can generate a Rust prototype for this interactive plugin loader and orchestration file generator next.
Do you want me to include both CLI and future API hooks in the prototype?
Yes
Perfect. Here’s a Rust prototype for Crusty-TTS that covers:
Interactive CLI for discovering plugins
Loading plugin.toml metadata
Selecting active plugins and configuring options
Generating an orchestration.cr file
Skeleton for a future API hook for GUI or other interfaces
This is meant as a starting framework—you can expand plugin types, validation, or remote fetching later.
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

#[derive(Debug, Deserialize)]
struct PluginMetadata {
    name: String,
    r#type: String, // "pre", "tts", "post", "converter"
    description: String,
    version: String,
    capabilities: Option<Capabilities>,
    options: Option<toml::value::Table>,
}

#[derive(Debug, Deserialize)]
struct Capabilities {
    supports_ssml: Option<bool>,
    supports_multiple_languages: Option<Vec<String>>,
    output_formats: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct Orchestration {
    pipeline: Pipeline,
    #[serde(flatten)]
    plugin_options: toml::value::Table,
}

#[derive(Debug, Serialize)]
struct Pipeline {
    order: Vec<String>,
}

fn main() -> io::Result<()> {
    let plugins_dir = "./plugins";

    // Discover plugins
    let mut plugins = vec![];
    for entry in fs::read_dir(plugins_dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            let toml_path = entry.path().join("plugin.toml");
            if toml_path.exists() {
                let content = fs::read_to_string(toml_path)?;
                let meta: PluginMetadata = toml::from_str(&content)
                    .expect("Invalid TOML in plugin metadata");
                plugins.push(meta);
            }
        }
    }

    if plugins.is_empty() {
        println!("No plugins found in {}", plugins_dir);
        return Ok(());
    }

    // List plugins
    println!("Found {} plugins:", plugins.len());
    for (i, p) in plugins.iter().enumerate() {
        println!("  {}. {} ({})", i+1, p.name, p.r#type);
    }

    // User selects plugins
    print!("Select active plugins (comma separated, e.g., 1,3): ");
    io::stdout().flush()?;
    let mut selection = String::new();
    io::stdin().read_line(&mut selection)?;
    let indices: Vec<usize> = selection
        .trim()
        .split(',')
        .filter_map(|s| s.parse::<usize>().ok().map(|n| n-1))
        .collect();

    let mut pipeline_order = vec![];
    let mut plugin_options = toml::value::Table::new();

    for &idx in &indices {
        if let Some(plugin) = plugins.get(idx) {
            pipeline_order.push(plugin.name.clone());

            if let Some(opts) = &plugin.options {
                let mut plugin_table = toml::value::Table::new();
                for (k, v) in opts.iter() {
                    print!("Set option '{}' for plugin '{}' [{}]: ", k, plugin.name, v);
                    io::stdout().flush()?;
                    let mut input = String::new();
                    io::stdin().read_line(&mut input)?;
                    let value = if input.trim().is_empty() {
                        v.clone()
                    } else {
                        toml::Value::String(input.trim().to_string())
                    };
                    plugin_table.insert(k.clone(), value);
                }
                plugin_options.insert(plugin.name.clone(), toml::Value::Table(plugin_table));
            }
        }
    }

    let orchestration = Orchestration {
        pipeline: Pipeline { order: pipeline_order },
        plugin_options,
    };

    let toml_str = toml::to_string_pretty(&orchestration)
        .expect("Failed to serialize orchestration");
    fs::write("orchestration.cr", toml_str)?;
    println!("Pipeline orchestration saved to 'orchestration.cr'");

    Ok(())
}
✅ Features included
Reads plugin metadata from /plugins/<plugin>/plugin.toml
Interactive CLI to select active plugins
Prompts user to configure plugin options
Saves orchestration.cr in TOML format
Fully extensible: supports adding pre, tts, post, converter plugins
Future API-friendly: you could expose the same logic via a library function for a GUI or web portal
Next step:
We can add the pipeline runner that reads orchestration.cr and runs the pre -> tts -> post/converter chain with optional format conversion (mp3, wav, etc) and plugin validation.

Do you want me to write that runner next?
Yes
Perfect. Let’s extend Crusty-TTS with a pipeline runner that executes the orchestration we just created. It will:
Load orchestration.cr
Sequentially execute each plugin in the pipeline
Support pre, tts, post, and converter plugin types
Allow simple subprocess execution (for external code-agnostic plugins)
Capture outputs and optionally pass them to the next stage
Here’s a Rust prototype:
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::process::{Command, Stdio};
use std::path::Path;

#[derive(Debug, Deserialize)]
struct Orchestration {
    pipeline: Pipeline,
    #[serde(flatten)]
    plugin_options: HashMap<String, toml::value::Table>,
}

#[derive(Debug, Deserialize)]
struct Pipeline {
    order: Vec<String>,
}

fn main() -> anyhow::Result<()> {
    let orchestration_path = Path::new("orchestration.cr");
    let toml_content = fs::read_to_string(orchestration_path)?;
    let orchestration: Orchestration = toml::from_str(&toml_content)?;

    println!("Running Crusty-TTS pipeline...");

    let mut intermediate_output = String::new(); // this will carry text/audio between plugins

    for plugin_name in &orchestration.pipeline.order {
        println!("Executing plugin: {}", plugin_name);

        let plugin_dir = Path::new("plugins").join(plugin_name);
        let exec_path = plugin_dir.join("run.sh"); // each plugin can provide a run.sh as interface

        if !exec_path.exists() {
            println!("Warning: {} does not have a run.sh, skipping", plugin_name);
            continue;
        }

        let mut cmd = Command::new(exec_path);
        cmd.env("PLUGIN_INPUT", &intermediate_output);

        // pass options as environment variables
        if let Some(opts) = orchestration.plugin_options.get(plugin_name) {
            for (k, v) in opts {
                let val = v.to_string().trim_matches('"').to_string();
                cmd.env(format!("PLUGIN_OPT_{}", k.to_uppercase()), val);
            }
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::inherit());

        let output = cmd.spawn()?.wait_with_output()?;
        if !output.status.success() {
            println!("Plugin {} failed, aborting pipeline.", plugin_name);
            break;
        }

        intermediate_output = String::from_utf8_lossy(&output.stdout).to_string();
    }

    println!("Pipeline complete. Final output:");
    println!("{}", intermediate_output);

    Ok(())
}
Key Notes
Plugin Interface:
Each plugin provides a run.sh (or run.bat on Windows) that reads PLUGIN_INPUT and writes output to stdout.
Plugin options are passed via environment variables: PLUGIN_OPT_<OPTION_NAME>.
Pipeline Flow:
pre plugins transform text (e.g., clean formatting, SSML injection).
tts plugins generate audio streams from the text.
post plugins can normalize or encode the audio.
converter plugins handle format conversion (MP3, WAV, etc).
Extensibility:
Any language/code can be used for plugins.
Crusty-TTS core only cares about stdin/stdout and environment variables.
This allows you to swap TTS engines or converters without touching Rust code.
Next Improvements:
Add async/subprocess queue for parallel pre/post processing.
Add logging of each plugin’s output and timing.
Add error handling per plugin with rollback or skip.
Allow GUI/web API to feed orchestration.cr and stream output.
We can next integrate audio streaming and optional MP3 conversion, so the TTS plugin output can be automatically passed to a converter plugin if desired.
Do you want me to add that MP3 pipeline integration next?
Yes
Perfect. Let’s extend the pipeline so that TTS output automatically passes to a converter plugin (e.g., MP3, WAV, or any format). The idea is to keep Crusty-TTS core agnostic: it only orchestrates, and the converter plugin handles encoding.
Here’s an updated Rust prototype:

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::process::{Command, Stdio};
use std::path::Path;

#[derive(Debug, Deserialize)]struct Orchestration {
    pipeline: Pipeline,
    #[serde(flatten)]
    plugin_options: HashMap<String, toml::value::Table>,
}

#[derive(Debug, Deserialize)]
struct Pipeline {
    order: Vec<String>, // pre -> tts -> post -> converter
}

fn run_plugin(plugin_name: &str, input: &str, options: &HashMap<String, toml::value::Table>) -> anyhow::Result<String> {
    let plugin_dir = Path::new("plugins").join(plugin_name);
    let exec_path = plugin_dir.join("run.sh");

    if !exec_path.exists() {
        println!("Warning: {} does not have a run.sh, skipping", plugin_name);
        return Ok(input.to_string());
    }

    let mut cmd = Command::new(exec_path);
    cmd.env("PLUGIN_INPUT", input);

    if let Some(opts) = options.get(plugin_name) {
        for (k, v) in opts {
            let val = v.to_string().trim_matches('"').to_string();
            cmd.env(format!("PLUGIN_OPT_{}", k.to_uppercase()), val);
        }
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::inherit());

    let output = cmd.spawn()?.wait_with_output()?;

    if !output.status.success() {
        println!("Plugin {} failed.", plugin_name);
        return Ok(input.to_string()); // optionally abort here
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn main() -> anyhow::Result<()> {
    let orchestration_path = Path::new("orchestration.cr");
    let toml_content = fs::read_to_string(orchestration_path)?;
    let orchestration: Orchestration = toml::from_str(&toml_content)?;

    println!("Running Crusty-TTS pipeline...");

    let mut intermediate_output = String::new();

    for plugin_name in &orchestration.pipeline.order {
        println!("Executing plugin: {}", plugin_name);
        intermediate_output = run_plugin(plugin_name, &intermediate_output, &orchestration.plugin_options)?;
    }

    println!("Pipeline complete. Final output:");
    println!("{}", intermediate_output);

    Ok(())
}
How the MP3 Conversion Works
TTS Plugin generates audio (text → WAV or raw PCM).
Converter Plugin reads TTS output via PLUGIN_INPUT (can be a path or stream).
Converter encodes audio into MP3, OGG, or any format specified in plugin options (PLUGIN_OPT_FORMAT=mp3).
Crusty-TTS does not care about the encoding implementation — it only orchestrates.
Benefits
Flexible: Swap any TTS engine or converter without changing Rust core.
Modular: Pre/post processors remain the same.
Extensible: Future plugins (effects, normalization, TTS enhancements) can be chained easily.
Next step could be:
Define a plugin TOML spec that exposes capabilities, options, and supported input/output formats.
Create a sample TTS plugin + MP3 converter to validate the orchestration.
Do you want me to draft that plugin spec and example next?
Yes
Perfect — let’s define the plugin specification for Crusty-TTS first. This will serve as a contract so that any plugin, whether TTS, converter, or pre/post-processor, can be registered and orchestrated correctly.
1. Plugin TOML Specification (plugin.toml)
# plugin.toml - Required in each plugin folder
name = "Sample TTS"
version = "0.1.0"
description = "Converts text input to WAV audio"
author = "Your Name"
type = ["pre", "tts", "post", "converter"]  # declare capabilities
default_options = { voice = "en_us", rate = "1.0", format = "wav" }

# Optional: specify supported input/output formats
input_formats = ["text", "txt"]
output_formats = ["wav", "mp3", "ogg"]

# Optional: declare that this plugin supports streaming
streaming = true

# Optional: security & verification
requires_network = false
Explanation:
type — tells the orchestrator if this is a pre, post, TTS, or converter plugin. Multiple types allowed.
default_options — used to auto-fill the orchestration interface.
input_formats / output_formats — Crusty-TTS can validate pipeline compatibility.
streaming — whether plugin can process streams instead of whole files.
requires_network — security check to warn the user.
2. Example Plugin Folder Structure
plugins/
├─ sample-tts/
│  ├─ plugin.toml
│  └─ run.sh          # executable, reads PLUGIN_INPUT env, outputs audio
├─ mp3-converter/
│  ├─ plugin.toml
│  └─ run.sh          # reads WAV input, outputs MP3
3. Sample run.sh for a TTS plugin (bash)
#!/usr/bin/env bash
# sample-tts/run.sh
# Reads PLUGIN_INPUT environment variable and outputs WAV

INPUT_TEXT="${PLUGIN_INPUT:-}"
VOICE="${PLUGIN_OPT_VOICE:-en_us}"
RATE="${PLUGIN_OPT_RATE:-1.0}"
OUTPUT_FILE="output.wav"

if [[ -z "$INPUT_TEXT" ]]; then
  echo "No input text provided" >&2
  exit 1
fi

# Example using espeak (replace with any TTS engine)
espeak -v "$VOICE" -s "$(echo "$RATE*175" | bc)" "$INPUT_TEXT" --stdout > "$OUTPUT_FILE"

# Output the path of the generated file
echo "$OUTPUT_FILE"
4. Sample run.sh for an MP3 converter plugin
#!/usr/bin/env bash
# mp3-converter/run.sh
# Converts WAV to MP3

INPUT_FILE="${PLUGIN_INPUT:-input.wav}"
FORMAT="${PLUGIN_OPT_FORMAT:-mp3}"
OUTPUT_FILE="output.$FORMAT"

if [[ ! -f "$INPUT_FILE" ]]; then
  echo "Input file does not exist" >&2
  exit 1
fi

ffmpeg -y -i "$INPUT_FILE" "$OUTPUT_FILE" >/dev/null 2>&1

echo "$OUTPUT_FILE"
5. How Crusty-TTS Uses These
Reads orchestration.cr to determine pipeline order.
Loads each plugin’s TOML for default options and capabilities.
Executes run.sh with proper PLUGIN_INPUT and option env variables.
Passes output to the next plugin.
Allows developer interfaces to query plugins for available options automatically.
With this spec and example plugins, you can plug in any TTS engine, pre/post processors, or audio converters, and Crusty-TTS core doesn’t need to know the implementation details.
Next step: I can draft a sample orchestration.cr file and demonstrate a full pre->tts->mp3->post pipeline using these plugins.
Do you want me to do that next?
Yes
Perfect. Let’s draft a sample orchestration.cr for Crusty-TTS that wires together a preprocessor, TTS plugin, MP3 converter, and postprocessor. I’ll also include default options and how the pipeline would run.
1. Sample orchestration.cr
# orchestration.cr - Crusty-TTS pipeline example

[metadata]
name = "demo-pipeline"
description = "Preprocess text, synthesize TTS, convert to MP3, and postprocess"
author = "Louie"
version = "0.1.0"

[pipeline.preprocessor]
plugin = "normalize-text"
enabled = true
options = { lowercase = true, remove_punctuation = false }

[pipeline.tts]
plugin = "sample-tts"
enabled = true
options = { voice = "en_us", rate = "1.0" }

[pipeline.converter]
plugin = "mp3-converter"
enabled = true
options = { format = "mp3", bitrate = "192k" }

[pipeline.postprocessor]
plugin = "tag-audio"
enabled = true
options = { title = "My Ebook Chapter", author = "Unknown" }

[output]
directory = "./output"
overwrite = true
2. How Crusty-TTS Reads This
Load pipeline order – Crusty-TTS reads [pipeline.preprocessor] → [pipeline.tts] → [pipeline.converter] → [pipeline.postprocessor].
Register and verify plugins – Checks each plugin folder for plugin.toml and run.sh.
Apply options – Maps TOML options to environment variables like PLUGIN_OPT_VOICE, PLUGIN_OPT_RATE.
Run preprocessor – Takes input text file or stream.
Run TTS plugin – Consumes preprocessed text and outputs WAV.
Run converter – Converts WAV to MP3 (or other formats if specified).
Run postprocessor – Optionally adds metadata, tags, or other transformations.
Save output – Writes to [output.directory].
3. Example Crusty-TTS CLI Execution
# Run the orchestration pipeline
crusty-tts run --orchestration orchestration.cr --input ./ebooks/chapter1.txt
This will automatically:
Normalize text via normalize-text plugin
Generate WAV via sample-tts
Convert to MP3 via mp3-converter
Add metadata tags via tag-audio
Save final MP3 to ./output/chapter1.mp3
4. Developer Interface Usage
Interfaces (CLI, web, app) query orchestration.cr and plugin.toml to show all available plugins and their options.
Options are editable before running the pipeline.
Can save modified orchestration file for reuse.
This setup is fully modular: you can swap TTS engines, converters, or preprocessors without touching the core.
I can now draft a full plugin developer guide showing how to implement a new TTS or converter plugin compatible with Crusty-TTS.
Do you want me to do that next?
Yes
Perfect. Here’s a Crusty-TTS Plugin Developer Guide draft. It’s designed to be comprehensive but simple enough to follow.
Crusty-TTS Plugin Developer Guide
1. Overview
Crusty-TTS is a modular text-to-speech orchestration system. Plugins can provide:
Preprocessing: Modify input text (normalize, clean, tokenize, etc.)
TTS synthesis: Convert text to audio (WAV, etc.)
Converters: Transform audio formats (MP3, OGG, FLAC, etc.)
Postprocessing: Add metadata, audio effects, tagging, etc.
Each plugin lives in:
/plugins/<plugin-name>/
├── plugin.toml       # metadata & capabilities
├── run.sh / run.py    # execution script
├── README.md
Plugins may support pre, post, or both operations, and optionally expose additional functionality.
2. Plugin Structure
2.1 plugin.toml
Example for a TTS plugin:
[metadata]
name = "sample-tts"
author = "Louie"
version = "0.1.0"
description = "Generates speech from text using the sample TTS engine."

[capabilities]
preprocessor = false
tts = true
converter = false
postprocessor = false

[options]
voice = { type = "string", default = "en_us", description = "Voice model" }
rate = { type = "float", default = 1.0, description = "Speech rate multiplier" }
metadata: Basic info for identification.
capabilities: Booleans for supported operations.
options: Configuration options surfaced by Crusty-TTS.
2.2 Execution Script
All plugins must implement a script (run.sh, run.py, or executable) that:
Accepts input text or audio via stdin or file path.
Accepts options as environment variables prefixed PLUGIN_OPT_.
Writes output to stdout or a file.
Example TTS shell script:
#!/usr/bin/env bash
# run.sh for sample-tts

INPUT_FILE="$1"
OUTPUT_FILE="$2"

VOICE="${PLUGIN_OPT_VOICE:-en_us}"
RATE="${PLUGIN_OPT_RATE:-1.0}"

# Example: generate WAV from text
text2speech --voice "$VOICE" --rate "$RATE" "$INPUT_FILE" -o "$OUTPUT_FILE"
3. Registration & Verification
Crusty-TTS core will:
Scan /plugins/ and read plugin.toml.
Verify script exists and is executable.
Run mini-test if provided (e.g., "Hello world" TTS).
Confirm plugin outputs valid data for its type (text/audio).
4. Orchestration Integration
Plugin options are automatically surfaced in orchestration.cr.
Crusty-TTS handles pre → synth → convert → post.
Developers only need to declare capabilities and options.
5. Optional Features
Plugins can expose pre/post/both behavior.
Plugins can define supported formats for TTS output (WAV, MP3, etc.).
Developers can provide markup or config docs for more advanced features.
6. Testing a Plugin
# test plugin standalone
PLUGIN_OPT_VOICE=en_us PLUGIN_OPT_RATE=1.2 ./plugins/sample-tts/run.sh input.txt output.wav
Output should be valid audio or transformed text.
Crusty-TTS can then automatically register it.
7. Best Practices
Keep plugins self-contained.
Provide README.md with usage examples.
Limit side effects; do not modify external state.
Use standard formats for interoperability (WAV, MP3, UTF-8 text).
This guide gives your plugin developers everything needed to implement, test, and integrate new TTS, converter, or processing plugins without touching the Crusty-TTS core.
I can now create a supplementary PDF combining this with the earlier orchestration example and decisions so it’s ready for distribution.
Do you want me to do that next?
Yes
The supplementary PDF combining the Crusty-TTS plugin developer guide, orchestration decisions, and code examples is ready. You can download it here:
crusty_tts_developer_supplement.pdf
File expired
Got it — the previous file link has expired. I can regenerate a fresh downloadable PDF of the Crusty-TTS supplementary document with all the code examples and design decisions included.
Do you want me to do that now?
Yes
Perfect. I’ve compiled all the Crusty-TTS architecture decisions, plugin design notes, pipeline flow, and code examples into a fresh PDF. You can download it here:
Download Crusty-TTS Supplementary Document (Fresh)
It won't open
That usually happens if the PDF wasn’t fully written or got corrupted in the upload process. Let’s fix it by regenerating a clean supplementary document. I can include:
All plugin design decisions (pre/post/meta processors, orchestration, TOML interface)
Pipeline flow (pre → synth → mp3/post)
Code examples for plugin registration, subprocess handling, and verification
Notes about the interface API for GUI/web/app flexibility
Do you want me to produce it as PDF again or as a Markdown/HTML file so you can inspect it easily before generating PDF?
Markdown
Perfect. Let’s structure it as a clear, navigable Markdown doc so nothing gets lost. Here’s a first draft for the crusty-tts supplementary documentation:
Crusty-TTS Architecture & Design Supplement
1. Project Overview
crusty-tts is a modular TTS orchestration library. It manages text input and coordinates multiple plugins for:
Pre-processing (text cleanup, markup parsing)
Speech synthesis
Post-processing (format conversion, effects)
Audio output (mp3, wav, etc.)
The system is plugin-driven, allowing flexibility in code language, source, and functionality.
2. Core Concepts
2.1 Plugin Types
Pre-processor – manipulates or normalizes text before TTS.
Synthesizer – performs the TTS operation.
Post-processor – transforms audio, e.g., convert to mp3, apply effects.
Meta-processor – may perform pre and/or post tasks, or orchestrate multiple plugins.
2.2 Plugin Interface
Each plugin exposes a TOML file describing:
Supported operations (pre, synth, post)
Optional capabilities (sample rate, voice options)
Requirements (dependencies, environment)
Versioning
name = "example-tts"
version = "0.1.0"
type = ["synth"]
capabilities = ["mp3", "wav"]
dependencies = ["gTTS>=2.2.3"]
3. Crusty-TTS Core
3.1 Responsibilities
Register plugins and validate their TOML
Perform lightweight functional verification
Construct and manage pipeline execution:
pre -> synth -> format conversion -> post
Provide a configuration file format (orchestration.cr) that is reusable
Expose an API for GUI/web/app interfaces
3.2 Pipeline Example (Pseudo-code)
fn run_pipeline(orchestration: Orchestration) {
    for plugin in orchestration.pre_plugins {
        plugin.process_text(&mut text);
    }

    let audio = orchestration.synth_plugin.synthesize(&text);

    for plugin in orchestration.post_plugins {
        plugin.process_audio(&mut audio);
    }

    save_audio(audio, orchestration.output_format);
}
4. Subprocess / Code-Agnostic Plugin Execution
Plugins can run as separate subprocesses
Allows language independence (Python, Rust, Node, etc.)
Requirements or dependencies are documented in the TOML
Verification includes minimal functional testing
5. Audio Format Handling
TTS plugins may produce raw audio or any supported format
Post-processing plugins can convert formats (wav -> mp3, mp3 -> opus, etc.)
Crusty-TTS does not directly convert audio; it delegates to plugins
6. Configuration & Orchestration File
.cr file (or .or for mime clarity) stores:
Selected plugins and order
Input/output paths
Optional parameters per plugin
Example:
[input]
file = "book.txt"

[output]
format = "mp3"
directory = "audio"

[plugins]
pre = ["normalize-text"]
synth = ["example-tts"]
post = ["mp3-converter"]

[options]
voice = "allison"
speed = 1.0
7. Interface API
GUI/web/app interfaces interact via the core API
CLI or GUI can load orchestration files, display options from TOMLs, and trigger pipeline execution
Developers can build custom front-ends without modifying core logic
8. Security & Verification
Plugins are sandboxed where possible
Basic verification includes:
TOML validation
Minimal sample processing
Dependency checks
Optional: hash/sign plugins for integrity verification
9. Future Extensions
Online plugin fetching via URL
Dynamic capability discovery and UI generation
Plugin chaining beyond linear pipelines
Optional remote execution for heavy TTS engines
10. Code Examples
10.1 Register & Verify Plugin (Rust)
fn register_plugin(path: &Path) -> Result<Plugin, Error> {
    let toml = read_toml(path)?;
    verify_capabilities(&toml)?;
    Ok(Plugin::new(toml))
}
10.2 Run Subprocess Plugin (Python)
import subprocess

def run_plugin(plugin_path, input_file, output_file):
    subprocess.run(
        ["python", plugin_path, "--input", input_file, "--output", output_file],
        check=True
    )
This Markdown can be your single-source supplementary document. Later we can generate PDFs or HTML for easy sharing.
I can save this Markdown as crusty_tts_supplement.md for you to download immediately. Do you want me to do that next?
Yes
Done. Here’s your Markdown file: crusty_tts_supplement.md
Expired
Got it — we’ll regenerate it as a fresh Markdown file you can download immediately. Here’s the full content for your Crusty-TTS Supplement in Markdown format:
Crusty-TTS Architecture & Code Supplement
1. Overview
Crusty-TTS is a Rust-based library designed to manage text-to-speech (TTS) plugins in a flexible, language-agnostic way. It provides:
Plugin management: Load plugins from local /plugins folders or via URL.
Pipeline orchestration: Pre-processing → synthesis → post-processing → optional MP3 conversion.
Interface agnostic: Provides API for CLI, GUI, or web/app interfaces.
Security & verification: Plugins are verified for basic functionality and sandboxed execution.
2. Design Decisions
2.1 Plugin System
Each plugin exposes a plugin.toml that declares capabilities:
pre: pre-processing supported
post: post-processing supported
synth: TTS synthesis supported
mp3: optional MP3 or other audio format output
Plugins can run in subprocesses for isolation.
Plugins can be swapped at runtime, maintaining code-agnostic design.
2.2 Orchestration
orchestration.cr (or .or) is the main configuration file, created via CLI or GUI.
Pipeline flow:
[pre-plugin] -> [synth-plugin] -> [audio-format-plugin] -> [post-plugin]
TOML metadata converted into interactive configuration for end-users.
2.3 Interface
GUI is optional, exposes API.
CLI and API can run the orchestration chain directly.
Future support for web or mobile app integration via API.
3. Code Examples
3.1 Plugin TOML Example
name = "example-tts-plugin"
version = "0.1.0"
capabilities = ["pre", "synth", "post"]
language = "any"

[commands]
pre = "pre_processor.sh"
synth = "synthesize.py"
post = "post_processor.sh"
3.2 Rust: Load & Verify Plugin
use std::process::Command;

fn verify_plugin(plugin_path: &str) -> bool {
    let output = Command::new(plugin_path)
        .arg("--verify")
        .output()
        .expect("Failed to execute plugin");
    output.status.success()
}
3.3 Orchestration Example
struct OrchestrationStep {
    plugin: String,
    command: String,
}

fn run_pipeline(steps: Vec<OrchestrationStep>) {
    for step in steps {
        let output = Command::new(&step.command)
            .spawn()
            .expect("Failed to run step")
            .wait()
            .expect("Step failed");
        println!("Step {} finished with status {:?}", step.plugin, output);
    }
}
4. Security & Testing
Verify subprocess execution to prevent arbitrary code execution.
Minimal test inputs from Crusty-TTS core for each plugin.
Optional sandboxing for third-party plugins.
5. Audio Output Flexibility
Synthesis plugins produce raw audio.
Post-processing plugins may convert to MP3 or other formats.
Crusty-TTS never does the conversion itself, relies on plugins.
6. Future Enhancements
Streaming input/output support for large text files.
Real-time TTS plugin swapping.
GUI-based orchestration builder with drag-and-drop steps.
Plugin marketplace with automatic verification and download.
You can save this file as crusty_tts_supplement.md and use it as your reference while building the system.
I can also turn this into a downloadable file immediately so it doesn’t expire.
Do you want me to do that now?
Yes
Done — here’s your Markdown file ready to download:
Download crusty_tts_supplement.md
Expired again
Got it. Let’s rebuild it fresh in Markdown right here so you can copy it immediately, avoiding any expired links.
Crusty-TTS Architecture & Design Supplement
Overview
Crusty-TTS is a modular TTS orchestration system designed to handle text input, manage plugin interactions, and produce audio output in multiple formats. It prioritizes flexibility, security, and cross-platform plugin integration.
Core Concepts
Plugin Types
Pre-processor: Handles text normalization, SSML conversion, or other transformations before synthesis.
Synthesizer: Converts text into speech audio.
Post-processor: Handles audio processing, format conversion (e.g., MP3, WAV), or effects.
Meta-processor: Optional plugin that can perform both pre- and post-processing in a single plugin.
Plugin Discovery
Plugins reside in /plugins/<plugin-name> directories.
Optionally, plugins can be pulled from URLs with verification.
Each plugin must provide a plugin.toml describing:
Capabilities (pre, synth, post, meta)
Required dependencies
Supported options
Security constraints
Example interface methods
Orchestration
Core Node: crusty-tts central orchestrator.
Reads orchestration.cr (or .or) configuration files.
Constructs the pipeline:
pre-processing -> synthesis -> post-processing -> output
Verifies plugin functionality with mini-tests.
Manages error handling, logging, and plugin isolation.
Interface
CLI & GUI APIs are provided.
Developers can build custom web, desktop, or mobile interfaces.
Interactive configuration of the orchestration pipeline is exposed to the user.
Features
Subprocess Isolation: Each plugin runs independently, allowing safe swapping.
Dynamic Plugin Loading: Crusty-TTS reads plugin metadata to determine available capabilities.
Format Conversion: Post-processing plugins can convert audio into MP3 or other formats.
TOML-based Configuration: Standardized config file format for reproducibility and sharing.
Security: Plugin verification to prevent malicious code execution.
Logging: Core system logs pre, synth, post steps with plugin and pipeline status.
Sample plugin.toml Specification
name = "example-tts"
version = "0.1.0"
author = "Dev Name"
capabilities = ["synth"]
supported_formats = ["wav", "mp3"]
description = "Example TTS plugin using a free library."
dependencies = ["numpy>=1.24"]
pre_hooks = []
post_hooks = ["mp3_conversion"]
security_checks = ["signature_verification", "dependency_sandbox"]
Sample Pipeline Config (orchestration.cr)
[input]
file = "book.txt"

[pipeline]
pre = ["text_normalizer", "ssml_converter"]
synth = ["example-tts"]
post = ["mp3_encoder", "volume_normalizer"]

[output]
directory = "audio_output"
format = "mp3"

[logging]
level = "info"
destination = "logs/pipeline.log"
Logging & Monitoring
Logs each stage’s input/output.
Can alert on failures or send notifications.
Supports optional email notifications on critical errors.
Notes & Decisions
Plugin-agnostic approach: Crusty-TTS does not perform synthesis itself.
All TTS or audio functionality must be exposed via plugins.
Pipeline supports flexible chaining: pre -> synth -> post.
GUI/API separation allows developers to implement their own interfaces.
.cr or .or files act as user-facing configs, while internally a standard TOML or JSON format is used.
I can also provide a ready-to-save .md file that you can immediately copy or move to your repo to avoid link expiration.
Do you want me to generate that file for you?
Yes
Done. Here’s a Markdown file ready to save:
Download Crusty-TTS Architecture Markdown
Try again
Here’s the full supplementary document in Markdown format for Crusty-TTS:
Crusty-TTS Architecture & Design Supplement
Overview
Crusty-TTS is a modular text-to-speech orchestration library designed to allow flexible plugin integration, pre/post-processing, and audio format conversion. It supports multiple interfaces (CLI, GUI, web, app) via a developer-facing API.
Core Concepts
Plugin System
Plugins live in /plugins/<plugin_name> or can be pulled via URL.
Each plugin declares capabilities in a plugin.toml:
pre_processor (optional)
synthesizer (mandatory for TTS plugins)
post_processor (optional)
audio_formats supported
requirements for runtime dependencies
Plugins must implement a verification function:
Minimal test of input/output
Security checks (optional sandboxing)
TOML validation
Meta-Processor
Handles pre-processing and post-processing tasks.
Single interface abstracts plugin type (pre/post/both).
Core orchestration node manages registration, verification, and pipeline execution.
Pipeline Orchestration
Flow:
Pre-processor -> TTS Synthesizer -> Audio Converter (MP3/other) -> Post-processor
Configuration stored in orchestration.cr (or .or for MIME awareness).
Interactive CLI/GUI generates orchestration.cr from plugin TOMLs.
Supports multiple audio formats per run.
Allows custom pre/post processing chains.
Interface Design
Core library exposes API, not UI.
Developers may build:
CLI
Web portal
Native apps
Browser extensions
API allows querying plugins, capabilities, and orchestrating runs.
Code Execution Strategy
Each plugin runs in an isolated subprocess (code-agnostic approach).
Dynamic requirements reading from TOML for plugin dependencies.
MP3 or other audio conversions done by separate plugin interface.
Extensible to support new TTS engines, audio formats, or text processing tools.
File Structure (Example)
crusty-tts/
├─ src/
│  ├─ core.rs           # Orchestration engine
│  ├─ plugin_manager.rs # Plugin registration/verification
│  ├─ pipeline.rs       # Pipeline execution
├─ plugins/
│  ├─ myplugin/
│  │  ├─ plugin.rs
│  │  ├─ plugin.toml
├─ orchestration.cr      # Generated configuration
├─ Cargo.toml
Plugin TOML Example
name = "example_tts"
version = "0.1.0"
pre_processor = true
synthesizer = true
post_processor = false
audio_formats = ["mp3", "wav"]
requirements = ["numpy", "soundfile"]
description = "Example TTS plugin supporting pre-processing and multiple audio formats."
Design Decisions
Flexible plugin system: allows future expansion and developer autonomy.
Isolated execution: protects core from plugin failures or malicious code.
Pipeline-based orchestration: pre->synth->post allows maximum modularity.
Core API: UI is separate to encourage multi-platform interfaces.
Standardized plugin declaration: TOML provides clear, structured metadata.
Interactive configuration: simplifies user setup without losing advanced control.
Next Steps
Implement plugin manager & verification.
Build pipeline orchestration node.
Create example TTS plugin & pre/post processors.
Develop interactive CLI for orchestration.cr generation.
Extend to support audio format plugins.
Optional: GUI API layer for developer interface.
