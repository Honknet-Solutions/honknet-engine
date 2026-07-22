#[cfg(test)]
mod tests {
    use honknet_testing::HeadlessHarness;
    #[tokio::test]
    async fn client_server_transport_round_trip() {
        assert!(
            HeadlessHarness::new()
                .round_trip(b"integration-state")
                .await
        )
    }
}
