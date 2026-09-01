use pwr_ext::view;

fn main() {
    let _ = view! {
        action_row {
            button {
                custom_id: "b1",
                label: "B1",
                { "not allowed" }
            }
        }
    };
}
