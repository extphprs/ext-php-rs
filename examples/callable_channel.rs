//! Example demonstrating thread-safe PHP callable invocation using the CSP
//! pattern.
//!
//! This example shows how to safely call PHP functions and closures from
//! background threads using the `CallableChannel` and `ClosureRegistry`.
//!
//! ## Key Concepts
//!
//! - **CallableChannel**: A message-passing channel for queuing PHP function
//!   calls
//! - **ClosureRegistry**: Stores PHP closures and provides thread-safe IDs
//! - **SerializedValue**: Thread-safe representation of PHP values
//!
//! ## Usage
//!
//! ```php
//! <?php
//! // Register a closure and get its ID
//! $closure = fn($x, $y) => $x + $y;
//! $closureId = register_callback($closure);
//!
//! // Start background work that will call the closure
//! start_background_computation($closureId, 10, 20);
//!
//! // Process any pending calls (call this periodically or in an event loop)
//! $processed = process_callbacks();
//!
//! // When done, unregister the closure
//! unregister_callback($closureId);
//! ```

#![allow(
    missing_docs,
    clippy::must_use_candidate,
    clippy::missing_safety_doc,
    clippy::missing_errors_doc,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::redundant_closure_for_method_calls
)]
#![cfg_attr(windows, feature(abi_vectorcall))]

use std::sync::Arc;
use std::thread;

use ext_php_rs::prelude::*;
use ext_php_rs::types::{CallableChannel, ClosureId, SerializedValue, ZendHashTable, Zval};

/// Global channel for callable requests.
/// In a real application, you might use a more sophisticated approach,
/// such as per-request channels or a channel pool.
static CHANNEL: std::sync::OnceLock<Arc<CallableChannel>> = std::sync::OnceLock::new();

fn get_channel() -> Arc<CallableChannel> {
    CHANNEL
        .get_or_init(|| Arc::new(CallableChannel::new()))
        .clone()
}

/// Register a PHP closure for later invocation from background threads.
///
/// Returns an integer ID that can be passed to background tasks.
/// The closure must be unregistered when no longer needed.
#[php_function]
pub fn register_callback(callable: &Zval) -> PhpResult<i64> {
    let channel = get_channel();
    match channel.registry().register(callable) {
        Some(id) => Ok(id.as_u64() as i64),
        None => Err(PhpException::default("Value is not callable".to_string())),
    }
}

/// Unregister a previously registered closure.
///
/// Returns true if the closure was found and removed.
#[php_function]
pub fn unregister_callback(id: i64) -> bool {
    let channel = get_channel();
    channel
        .registry()
        .unregister(ClosureId::from_u64(id as u64))
}

/// Process all pending callback requests.
///
/// This must be called on the main PHP thread. It executes all queued
/// function/closure calls and returns the number of requests processed.
///
/// Call this periodically in your event loop or after spawning background work.
#[php_function]
pub fn process_callbacks() -> i64 {
    get_channel().process_pending() as i64
}

/// Check if there are pending callback requests.
#[php_function]
pub fn has_pending_callbacks() -> bool {
    get_channel().has_pending()
}

/// Get the number of pending callback requests.
#[php_function]
pub fn pending_callback_count() -> i64 {
    get_channel().pending_count() as i64
}

/// Demonstrate calling a named PHP function from a background thread.
///
/// This spawns a thread that queues a call to the specified function,
/// then returns immediately. Call `process_callbacks()` to execute the queued
/// call.
#[php_function]
pub fn call_function_async(function_name: String, args: &ZendHashTable) -> PhpResult<bool> {
    let channel = get_channel();

    // Convert PHP array to SerializedValue vector
    let serialized_args: Vec<SerializedValue> = args
        .iter()
        .filter_map(|(_, val)| SerializedValue::from_zval(val))
        .collect();

    // Spawn a background thread
    thread::spawn(move || {
        // Queue the call - this is thread-safe
        let _handle = channel.queue_call(function_name, serialized_args);
        // In a real application, you might store the handle to retrieve the
        // result
    });

    Ok(true)
}

/// Demonstrate calling a registered closure from a background thread.
///
/// This spawns a thread that queues a call to the closure with the given ID.
#[php_function]
pub fn call_closure_async(closure_id: i64, args: &ZendHashTable) -> PhpResult<bool> {
    let channel = get_channel();
    let id = ClosureId::from_u64(closure_id as u64);

    // Convert PHP array to SerializedValue vector
    let serialized_args: Vec<SerializedValue> = args
        .iter()
        .filter_map(|(_, val)| SerializedValue::from_zval(val))
        .collect();

    // Spawn a background thread
    thread::spawn(move || {
        let _handle = channel.queue_closure_call(id, serialized_args);
    });

    Ok(true)
}

/// Demonstrate synchronous function call with result retrieval.
///
/// This queues a call, processes it immediately, and demonstrates
/// the full round-trip.
#[php_function]
pub fn call_function_sync(function_name: String, args: &ZendHashTable) -> PhpResult<Zval> {
    let channel = get_channel();

    // Convert PHP array to SerializedValue vector
    let serialized_args: Vec<SerializedValue> = args
        .iter()
        .filter_map(|(_, val)| SerializedValue::from_zval(val))
        .collect();

    // Queue the call
    let handle = channel.queue_call(&function_name, serialized_args);

    // Process pending calls (executes our queued call)
    channel.process_pending();

    // Get the result
    match handle.wait() {
        Ok(result) => result.to_zval().map_err(|e| e.into()),
        Err(e) => Err(PhpException::default(format!("Call failed: {e}"))),
    }
}

/// Demonstrate synchronous closure call with result retrieval.
#[php_function]
pub fn call_closure_sync(closure_id: i64, args: &ZendHashTable) -> PhpResult<Zval> {
    let channel = get_channel();
    let id = ClosureId::from_u64(closure_id as u64);

    // Convert PHP array to SerializedValue vector
    let serialized_args: Vec<SerializedValue> = args
        .iter()
        .filter_map(|(_, val)| SerializedValue::from_zval(val))
        .collect();

    // Queue the call
    let handle = channel.queue_closure_call(id, serialized_args);

    // Process pending calls
    channel.process_pending();

    // Get the result
    match handle.wait() {
        Ok(result) => result.to_zval().map_err(|e| e.into()),
        Err(e) => Err(PhpException::default(format!("Closure call failed: {e}"))),
    }
}

/// Spawn multiple background tasks that all call the same closure.
///
/// Demonstrates concurrent access to the channel from multiple threads.
#[php_function]
pub fn parallel_closure_calls(closure_id: i64, values: &ZendHashTable) -> PhpResult<i64> {
    let channel = get_channel();
    let id = ClosureId::from_u64(closure_id as u64);

    let mut handles = Vec::new();

    // Spawn a thread for each value
    for (_, val) in values {
        if let Some(serialized) = SerializedValue::from_zval(val) {
            let channel_clone = channel.clone();
            let handle =
                thread::spawn(move || channel_clone.queue_closure_call(id, vec![serialized]));
            handles.push(handle);
        }
    }

    // Wait for all threads to finish queueing
    let call_handles: Vec<_> = handles.into_iter().filter_map(|h| h.join().ok()).collect();

    let count = call_handles.len() as i64;

    // Note: The calls are queued but not processed yet
    // Call process_callbacks() to execute them

    Ok(count)
}

/// Get the number of registered closures.
#[php_function]
pub fn registered_closure_count() -> i64 {
    get_channel().registry().len() as i64
}

#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module
        .function(wrap_function!(register_callback))
        .function(wrap_function!(unregister_callback))
        .function(wrap_function!(process_callbacks))
        .function(wrap_function!(has_pending_callbacks))
        .function(wrap_function!(pending_callback_count))
        .function(wrap_function!(call_function_async))
        .function(wrap_function!(call_closure_async))
        .function(wrap_function!(call_function_sync))
        .function(wrap_function!(call_closure_sync))
        .function(wrap_function!(parallel_closure_calls))
        .function(wrap_function!(registered_closure_count))
}

fn main() {}
