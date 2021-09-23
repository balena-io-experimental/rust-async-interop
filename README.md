# rust-async-interop
Rust Tokio and GLib async runtimes interoperability

Asynchronous libnm-rs NetworkManager communication requires a GLib runtime event loop. On the other hand lots of the popular web application frameworks need a Tokio runtime. This playground example runs the Tokio and GLib runtimes in separate threads while maintaining the ergonomics of async usage throughout the code.

Tokio runtime runs in the main thread and axum is used as an example web framework on top of it. The GLib runtime is spawned in a new thread on start. Communication between the two threads is done through GLib and Tokio channels.

On start a GLib channel is created and its receiver is passed when the GLib thread is spawned. Then the axum web application is started on `localhost:3000`.

The GLib thread registers a callback with that receiver and starts its event loop.

The GLib sender is kept in the main thread and is stored as a state for the axum request handlers.

When incoming axum web request is received on the main thread a new Tokio oneshot channel is created. The oneshot channel sender and the NetworkManager command are then sent through the GLib channel sender. They are received on the GLib thread by the callback of the GLib receiver. The NetworkManager command is then dispatched and libnm is invoked. On completion the result is passed back to the main thread through the Tokio oneshot sender. The oneshot receiver obtains the result and passes it to the web response.

Example:
 * Run the application with `cargo run`
 * From another terminal run `curl localhost:3000/list-connections` and `curl localhost:3000/check-connectivity`
