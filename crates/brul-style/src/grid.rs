use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseGridError;

impl fmt::Display for ParseGridError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid grid value")
    }
}

impl std::error::Error for ParseGridError {}

#[derive(Debug, Clone, PartialEq)]
pub enum TrackSize {
    Px(f64),
    Percent(f64),
    Fr(f64),
    Minmax(Box<TrackSize>, Box<TrackSize>),
}

impl FromStr for TrackSize {
    type Err = ParseGridError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let text = input.trim();
        if let Some(inner) = text
            .strip_prefix("minmax(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let mut parts = split(inner, ',');
            let min = parts.next().ok_or(ParseGridError)?.parse()?;
            let max = parts.next().ok_or(ParseGridError)?.parse()?;
            if parts.next().is_some() {
                return Err(ParseGridError);
            }
            return Ok(Self::Minmax(Box::new(min), Box::new(max)));
        }
        if let Some(fr) = text.strip_suffix("fr") {
            return fr.trim().parse().map(Self::Fr).map_err(|_| ParseGridError);
        }
        if let Some(px) = text.strip_suffix("px") {
            return px.trim().parse().map(Self::Px).map_err(|_| ParseGridError);
        }
        if let Some(percent) = text.strip_suffix('%') {
            return percent
                .trim()
                .parse()
                .map(Self::Percent)
                .map_err(|_| ParseGridError);
        }
        Err(ParseGridError)
    }
}

pub fn parse_tracks(input: &str) -> Result<Vec<TrackSize>, ParseGridError> {
    let tracks = split(input, ' ')
        .filter(|part| !part.is_empty())
        .map(TrackSize::from_str)
        .collect::<Result<Vec<_>, _>>()?;
    if tracks.is_empty() {
        return Err(ParseGridError);
    }
    Ok(tracks)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinePlacement {
    Auto,
    Line(i32),
    Span(u16),
}

impl FromStr for LinePlacement {
    type Err = ParseGridError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let text = input.trim();
        if text == "auto" {
            return Ok(Self::Auto);
        }
        if let Some(count) = text.strip_prefix("span ") {
            return count
                .trim()
                .parse()
                .map(Self::Span)
                .map_err(|_| ParseGridError);
        }
        text.parse().map(Self::Line).map_err(|_| ParseGridError)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridLine {
    pub start: LinePlacement,
    pub end: Option<LinePlacement>,
}

impl FromStr for GridLine {
    type Err = ParseGridError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let mut parts = split(input, '/');
        let start = parts.next().ok_or(ParseGridError)?.parse()?;
        let end = match parts.next() {
            Some(part) => Some(part.parse()?),
            None => None,
        };
        if parts.next().is_some() {
            return Err(ParseGridError);
        }
        Ok(Self { start, end })
    }
}

fn split(input: &str, separator: char) -> impl Iterator<Item = &str> {
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut parts = Vec::new();
    for (index, character) in input.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            character if character == separator && depth == 0 => {
                parts.push(input[start..index].trim());
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(input[start..].trim());
    parts.into_iter()
}

#[cfg(test)]
mod tests {
    use super::{GridLine, LinePlacement, TrackSize, parse_tracks};

    #[test]
    fn a_track_list_accepts_lengths_fr_and_minmax() {
        let tracks = parse_tracks("100px 1fr minmax(80px, 2fr) 25%").unwrap();
        assert_eq!(
            tracks,
            [
                TrackSize::Px(100.0),
                TrackSize::Fr(1.0),
                TrackSize::Minmax(Box::new(TrackSize::Px(80.0)), Box::new(TrackSize::Fr(2.0))),
                TrackSize::Percent(25.0),
            ]
        );
    }

    #[test]
    fn a_grid_line_accepts_auto_lines_and_spans() {
        assert_eq!(
            "auto".parse::<GridLine>().unwrap().start,
            LinePlacement::Auto
        );
        assert_eq!(
            "2 / span 3".parse::<GridLine>().unwrap(),
            GridLine {
                start: LinePlacement::Line(2),
                end: Some(LinePlacement::Span(3)),
            }
        );
        assert_eq!(
            "-1".parse::<GridLine>().unwrap().start,
            LinePlacement::Line(-1)
        );
    }

    #[test]
    fn malformed_grid_values_are_rejected() {
        assert!("1fr 2fr,".parse::<TrackSize>().is_err());
        assert!(parse_tracks("").is_err());
        assert!("2 / 3 / 4".parse::<GridLine>().is_err());
    }
}
