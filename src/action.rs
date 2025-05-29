#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Quit,
    NavigateToHome,
    NavigateToDevice,
    NavigateToSniffer,
    DeviceSelected(String),
    ApplyFilter(String),
    Handled,
    PacketSelected(usize), // New action for packet selection
}
