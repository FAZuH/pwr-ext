use pwr_ext::view;
fn main() {
    let _ = view! { action_row { button { custom_id: "1", url: "https://example.test", label: "a" } } };
}
