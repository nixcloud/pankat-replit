use crate::button::{self, Button};
use crate::datebox::{date, datebox, time};
use js_sys::Date;
use sauron::dom::component;
use sauron::dom::stateful_component;
use sauron::prelude::*;

#[derive(Default)]
pub enum Msg {
    Click,
    DoubleClick,
    Clock,
    ChangeName(String),
    ChangeBiography(String),
    ChangeThought(String),
    BtnMsg(button::Msg),
    DateChanged(String),
    #[default]
    NoOp,
}

pub struct App {
    click_count: u32,
    double_clicks: u32,
    date: Date,
    name: String,
    biography: String,
    thought: Option<String>,
    btn: Button,
    selected_date: Option<String>,
}

impl App {
    pub fn new(click_count: u32) -> App {
        App {
            click_count,
            double_clicks: 0,
            date: Date::new_0(),
            name: String::new(),
            biography: String::new(),
            thought: None,
            btn: Button::default(),
            selected_date: None,
        }
    }
}

impl Application for App {
    type MSG = Msg;

    fn init(&mut self) -> Cmd<Msg> {
        Time::every(5_000, || Msg::Clock)
    }

    fn update(&mut self, msg: Msg) -> Cmd<Msg> {
        match msg {
            Msg::Click => {
                self.click_count += 1;
                log::info!("click count: {}", self.click_count);
                Cmd::none()
            }
            Msg::DoubleClick => {
                self.double_clicks += 1;
                Cmd::none()
            }
            Msg::Clock => {
                self.date = Date::new_0();
                Cmd::none()
            }
            Msg::ChangeName(name) => {
                self.name = name;
                Cmd::none()
            }
            Msg::ChangeBiography(bio) => {
                self.biography = bio;
                Cmd::none()
            }
            Msg::ChangeThought(thought) => {
                if !thought.is_empty() {
                    self.thought = Some(thought);
                } else {
                    self.thought = None;
                }
                Cmd::none()
            }
            Msg::BtnMsg(bmsg) => {
                let effects = Component::update(&mut self.btn, bmsg);
                effects.map_msg(Msg::BtnMsg).into()
            }
            Msg::DateChanged(date_time) => {
                log::info!("IN APP.RS the DATE is now changed...: {date_time}");
                self.selected_date = Some(date_time);
                Cmd::none()
            }
            Msg::NoOp => Cmd::none(),
        }
    }

    view! {
        <div class="some-class" id="some-id" {attr("data-id", 1)} {style!{"font-family": "monospace"}}>
            <div id="current-time">{text!("Today is {}",self.date.to_locale_string("en-GB", &JsValue::undefined()))}</div>
            <div>
                    "Your name is: "
                    <input type="text"
                            on_input=|event: InputEvent| Msg::ChangeName(event.value())
                            placeholder="John Smith"
                            data-attr=format!("Hello{}", &self.name)
                    />
                    <button on:click=|event: MouseEvent| {
                            trace!("Clicked at ({},{})", event.x(), event.y());
                            Msg::Click}>
                        Click me!
                    </button>
                    <button on_dblclick=|event: MouseEvent| {
                            trace!("Double clicked at ({},{})", event.x(), event.y());
                            Msg::DoubleClick}>
                        {text!("DoubleClicks {}", self.double_clicks)}
                    </button>
            </div>
            <p>
                {text!("Hello {}!", self.name)}
                {if self.click_count > 0 {
                    text!(
                        ", You've clicked on that button for {} time{}",
                        self.click_count,
                        if self.click_count > 1 { "s" } else { "" }
                    )
                } else {
                    text("here..")
                }}
            </p>
            <div>
                    <p>Tell us something about yourself</p>
                    <div>
                        <textarea rows=10 cols=80
                                on_input=|event: InputEvent| {
                                    Msg::ChangeBiography(event.value())
                                }
                                placeholder="I'm a..."
                        />
                    </div>
                    <p>{text!("{}", self.biography)}</p>
            </div>
            <div>
                    "What are you thinking right now?"
                    <input type="text"
                            on_change=|event: InputEvent| Msg::ChangeThought(event.value())
                            placeholder="Elephants..."
                    />
                    {if let Some(thought) = &self.thought {
                        text(format!("Hmmn {}... Interesting.", thought))
                    } else {
                        node!{<span></span>}
                    }}
            </div>
            <ul class="some-list">
                {for i in 0..10{
                    node!{
                        <li key=i>{text!("i: {}", i)}</li>
                    }
                }}
            </ul>
            <ul class="fruits">
                {
                    for i in ["apple", "orange", "grapes"]{
                    node!{
                        <li key=i>{text!("i: {}", i)}</li>
                    }
                }}
            </ul>
            <div>
                {component(&self.btn).map_msg(Msg::BtnMsg)}
            </div>
            <div>
                {stateful_component(Button::default(), [], [text!("External child of btn stateful_component: {}", self.click_count)])}
            </div>
            <div>
                {datebox([date("2022-07-07"), time("07:07"),
                    on_input(|ie|{
                        log::info!("on_input is triggered...");
                        Msg::DateChanged(ie.value())
                    }),
                    on_change(|ie|{
                        log::info!("on_change is triggered...");
                        Msg::DateChanged(ie.value())
                    })],[])}
                {
                    if let Some(selected_date) = self.selected_date.as_ref(){
                        text!("You've selected date: {}", selected_date)
                    }else{
                        text!("You haven't selected a date")
                    }
                }
            </div>
        </div>
    }

    fn stylesheet() -> Vec<String> {
        vec![jss! {
            "body": {
                font_family: "Fira Sans, Courier New, Courier, Lucida Sans Typewriter, Lucida Typewriter, monospace",
            }
        }]
    }
}
