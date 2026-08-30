use pwr_ext::view;
fn main() {
    let _ = view! {
        action_row {
            button { custom_id: "1", label: "a" }
            button { custom_id: "2", label: "b" }
            button { custom_id: "3", label: "c" }
            button { custom_id: "4", label: "d" }
            button { custom_id: "5", label: "e" }
            button { custom_id: "6", label: "f" }
        }
    };
}
