use error::MasqueError;
use std::str::FromStr;

type EventResult<T> = Result<T, MasqueError>;

#[derive(Debug, PartialEq)]
pub struct Event {
    id: Option<String>,
    event: Option<String>,
    data: String,
}

impl Event {
    pub fn new<T, S>(id: Option<T>, event: Option<S>, data: String) -> Event
    where
        T: Into<String>,
        S: Into<String>,
    {
        Event {
            id: id.map(|val| val.into()),
            event: event.map(|val| val.into()),
            data: data,
        }
    }

    pub fn id(&self) -> &Option<String> {
        &self.id
    }

    pub fn event(&self) -> &Option<String> {
        &self.event
    }

    pub fn data(&self) -> &str {
        self.data.as_str()
    }
}

#[derive(Debug, PartialEq)]
pub enum EventTag {
    Id,
    Event,
    Data,
    Retry,
    End,
}

impl FromStr for EventTag {
    type Err = MasqueError;

    fn from_str(s: &str) -> EventResult<EventTag> {
        match s {
            "id" => Ok(EventTag::Id),
            "event" => Ok(EventTag::Event),
            "data" => Ok(EventTag::Data),
            "retry" => Ok(EventTag::Retry),
            _ => Err(MasqueError::InvalidEventTag),
        }
    }
}

#[derive(Debug, PartialEq)]
struct EventLine {
    pub tag: EventTag,
    pub data: String,
}

impl FromStr for EventLine {
    type Err = MasqueError;

    fn from_str(s: &str) -> EventResult<EventLine> {
        if let Some(pos) = s.find(':') {
            let (string_tag, value) = s.split_at(pos);
            let tag = string_tag.parse::<EventTag>()?;

            Ok(EventLine {
                tag: tag,
                data: value[1..].trim_left().trim_right_matches("\n").into(),
            })
        } else {
            Ok(EventLine {
                tag: EventTag::End,
                data: "".into(),
            })
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum EventLineResponse {
    Continue(EventBuilder),
    Complete(Event),
}

#[derive(Clone, Debug, PartialEq)]
pub struct EventBuilder {
    id: Option<String>,
    event: Option<String>,
    data: String,
}

impl EventBuilder {
    pub fn new() -> EventBuilder {
        EventBuilder {
            id: None,
            event: None,
            data: "".into(),
        }
    }

    pub fn id<T>(mut self, id: Option<T>) -> EventBuilder
    where
        T: Into<String>,
    {
        self.id = id.map(|val| val.into());
        self
    }

    pub fn event<T>(mut self, event: Option<T>) -> EventBuilder
    where
        T: Into<String>,
    {
        self.event = event.map(|val| val.into());
        self
    }

    pub fn data<T>(mut self, data: T) -> EventBuilder
    where
        T: Into<String>,
    {
        self.data = data.into();
        self
    }

    pub fn extend_data<T>(mut self, data: T) -> EventBuilder
    where
        T: AsRef<str>,
    {
        if self.data == "" {
            self.data = data.as_ref().to_string();
        } else {
            self.data = self.data + "\n" + data.as_ref();
        }

        self
    }

    pub fn read_in_line<T>(self, line: T) -> EventLineResponse
    where
        T: AsRef<str>,
    {
        if let Ok(e) = line.as_ref().parse::<EventLine>() {
            match e.tag {
                EventTag::Id => EventLineResponse::Continue(self.id(Some(e.data))),
                EventTag::Event => EventLineResponse::Continue(self.event(Some(e.data))),
                EventTag::Data => EventLineResponse::Continue(self.extend_data(e.data)),
                EventTag::Retry => EventLineResponse::Continue(self),
                EventTag::End => EventLineResponse::Complete(self.build()),
            }
        } else {
            EventLineResponse::Continue(self)
        }
    }

    pub fn read_in_lines<T>(mut self, line: Vec<T>) -> EventLineResponse
    where
        T: AsRef<str>,
    {
        for l in line {
            match self.read_in_line(l) {
                EventLineResponse::Continue(builder) => self = builder,
                EventLineResponse::Complete(event) => return EventLineResponse::Complete(event),
            }
        }

        EventLineResponse::Continue(self)
    }

    pub fn build(self) -> Event {
        Event::new(self.id, self.event, self.data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parses_event_types() {
        let id = "id: the-event-uuid\n";
        let event = "event: thetype\n";
        let data = "data: some event data\n";
        // let retry

        let id_p = id.parse::<EventLine>().unwrap();
        let event_p = event.parse::<EventLine>().unwrap();
        let data_p = data.parse::<EventLine>().unwrap();

        assert_eq!(id_p.tag, EventTag::Id);
        assert_eq!(id_p.data.as_str(), "the-event-uuid");
        assert_eq!(event_p.tag, EventTag::Event);
        assert_eq!(event_p.data.as_str(), "thetype");
        assert_eq!(data_p.tag, EventTag::Data);
        assert_eq!(data_p.data.as_str(), "some event data");
    }

    #[test]
    fn test_parses_ending_line() {
        let end = "\n";

        let end_p = end.parse::<EventLine>().unwrap();

        assert_eq!(end_p.tag, EventTag::End);
    }

    #[test]
    fn test_concats_multiple_data_lines_with_newline() {
        let b = EventBuilder::new();

        match b.read_in_lines(vec!["data: line1\n", "data: line2\n"]) {
            EventLineResponse::Continue(builder) => {
                let ev = builder.build();
                assert_eq!(ev.data(), "line1\nline2");
            }
            EventLineResponse::Complete(_) => panic!("Builder not found"),
        }
    }

    #[test]
    fn test_omits_lines_after_ending_line() {
        let b = EventBuilder::new();

        let resp = b.read_in_lines(vec![
            "id: the-event-uuid\n",
            "event: thetype\n",
            "data: the first data\n",
            "data: the second data\n",
            "\n",
            "data: the third data\n",
            "data: the fourth data\n",
        ]);

        match resp {
            EventLineResponse::Continue(_) => panic!("Builder did not terminate"),
            EventLineResponse::Complete(ev) => {
                assert_eq!(ev.data(), "the first data\nthe second data");
            }
        }
    }

    #[test]
    fn test_builds_on_ending_line() {
        let b = EventBuilder::new();

        let resp = b.read_in_lines(vec![
            "id: the-event-uuid\n",
            "event: thetype\n",
            "data: the first data\n",
            "data: the rest of the data\n",
            "\n",
        ]);

        match resp {
            EventLineResponse::Continue(_) => panic!("Builder did not terminate"),
            EventLineResponse::Complete(_) => (),
        }
    }
}
