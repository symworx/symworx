// Copyright (c) 2026 SymWorx
// Licensed under the Apache License, Version 2.0.

#![cfg(feature = "supervision")]

use tokio::signal;
use tokio_util::sync::CancellationToken;

/// Creates a cancellation token that gets cancelled on Ctrl+C or SIGTERM.
pub async fn graceful_shutdown() -> CancellationToken {
    let token = CancellationToken::new();
    let cloned = token.clone();

    tokio::spawn(async move {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        cloned.cancel();
    });

    token
}
