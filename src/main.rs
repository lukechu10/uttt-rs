use sycamore::prelude::*;

#[component(App<G>)]
fn app() -> View<G> {
    view! {
        div(class="container mx-auto") {
            h1(class="text-xl") { "Ultimate TicTacToe" }
        }
    }
}

fn main() {
    sycamore::render(|| {
        view! {
            App()
        }
    })
}
