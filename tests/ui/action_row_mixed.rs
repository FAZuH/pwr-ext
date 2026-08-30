use pwr_ext::view;
fn main() {
    let _ = view! {
        action_row {
            button { custom_id: "1", label: "a" }
            select_menu { custom_id: "2" }
        }
    };
}
