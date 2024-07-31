use maud::{html, Markup};

pub fn render() -> Markup {
    // generates a vec of 8.5->23 in 0.5 increments. this will later be generated
    // dynamically based on the earliest and latest start time in a person's schedule
    let timeslots: Vec<f64> = Vec::from_iter((17..46).map(|n| n as f64 * 0.5));

    let days: Vec<&str> = vec!["monday", "tuesday", "wednesday", "thursday", "friday"];

    html!(
        div id="calendar" class="w-full h-full flex" {
            div class="w-12 flex flex-col" {
                div class="text-[0.5rem] border-b-2 box-content grow" { "time" }
                @for time in &timeslots {
                    div class="text-[0.25rem] border-b-2 box-content grow" { (time) }
                }
            }
            @for day in &days {
                div class="grow flex flex-col" {
                    div class="text-[0.5rem] border-b-2 box-content grow flex justify-center items-center" { (day) }
                    @for _ in &timeslots {
                        div class="text-[0.25rem] border-b-2 box-content grow" { "." }
                    }
                }
            }
        }
    )
}
