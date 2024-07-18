use hypertext::{html_elements, rsx, GlobalAttributes, Renderable};

pub fn c(content: impl Renderable) -> impl Renderable {
    rsx! {
        <button class="border-2">
            {content}
        </button>
    }
}
