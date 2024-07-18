use hypertext::{html_elements, rsx, Renderable};

pub mod button;

pub fn base(content: impl Renderable) -> impl Renderable {
    rsx! {
        <html>
            <head>
                <title>"Course Scheduler"</title>
                <script src="https://unpkg.com/htmx.org@2.0.1"></script>
                <script src="https://cdn.tailwindcss.com"></script>
            </head>
            <body>
                {content}
            </body>
        </html>
    }
}
