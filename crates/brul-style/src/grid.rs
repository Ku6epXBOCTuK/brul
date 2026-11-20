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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatCount {
    Count(u16),
    AutoFill,
    AutoFit,
}

impl FromStr for RepeatCount {
    type Err = ParseGridError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.trim() {
            "auto-fill" => Ok(Self::AutoFill),
            "auto-fit" => Ok(Self::AutoFit),
            text => text
                .parse()
                .ok()
                .filter(|&count| count > 0)
                .map(Self::Count)
                .ok_or(ParseGridError),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Area {
    pub name: String,
    pub row_start: u16,
    pub row_end: u16,
    pub column_start: u16,
    pub column_end: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AreaTemplate {
    pub rows: Vec<Vec<Option<String>>>,
}

impl AreaTemplate {
    pub fn parse(rows: &[String]) -> Result<Self, ParseGridError> {
        let parsed = rows
            .iter()
            .map(|row| {
                row.split_whitespace()
                    .map(|cell| match cell {
                        "." => Ok(None),
                        name if valid_area_name(name) => Ok(Some(name.to_owned())),
                        _ => Err(ParseGridError),
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let width = parsed.first().ok_or(ParseGridError)?.len();
        if width == 0
            || parsed.len() > u16::MAX as usize
            || width > u16::MAX as usize
            || parsed.iter().any(|row| row.len() != width)
        {
            return Err(ParseGridError);
        }
        let template = Self { rows: parsed };
        template.areas()?;
        Ok(template)
    }

    pub fn row_count(&self) -> u16 {
        self.rows.len() as u16
    }

    pub fn column_count(&self) -> u16 {
        self.rows.first().map_or(0, |row| row.len()) as u16
    }

    pub fn areas(&self) -> Result<Vec<Area>, ParseGridError> {
        let mut names = Vec::new();
        for cell in self.rows.iter().flatten().flatten() {
            if !names.contains(cell) {
                names.push(cell.clone());
            }
        }
        let mut areas = Vec::new();
        for name in names {
            let mut first_row = usize::MAX;
            let mut last_row = 0;
            let mut first_column = usize::MAX;
            let mut last_column = 0;
            for (row, cells) in self.rows.iter().enumerate() {
                for (column, cell) in cells.iter().enumerate() {
                    if cell.as_deref() == Some(name.as_str()) {
                        first_row = first_row.min(row);
                        last_row = last_row.max(row);
                        first_column = first_column.min(column);
                        last_column = last_column.max(column);
                    }
                }
            }
            let height = last_row - first_row + 1;
            let width = last_column - first_column + 1;
            let filled = self.rows[first_row..=last_row]
                .iter()
                .flat_map(|cells| &cells[first_column..=last_column])
                .filter(|cell| cell.as_deref() == Some(name.as_str()))
                .count();
            if filled != height * width {
                return Err(ParseGridError);
            }
            areas.push(Area {
                name,
                row_start: first_row as u16 + 1,
                row_end: last_row as u16 + 2,
                column_start: first_column as u16 + 1,
                column_end: last_column as u16 + 2,
            });
        }
        Ok(areas)
    }
}

fn valid_area_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

#[derive(Debug, Clone, PartialEq)]
pub enum TrackSize {
    Px(f64),
    Percent(f64),
    Fr(f64),
    Minmax(Box<TrackSize>, Box<TrackSize>),
    Repeat(RepeatCount, Vec<TrackSize>),
}

impl FromStr for TrackSize {
    type Err = ParseGridError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let text = input.trim();
        if let Some(inner) = text
            .strip_prefix("repeat(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let mut parts = split(inner, ',');
            let count = parts.next().ok_or(ParseGridError)?.parse()?;
            let tracks = parts.collect::<Vec<_>>().join(" ");
            return Ok(Self::Repeat(count, parse_tracks(&tracks)?));
        }
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LinePlacement {
    #[default]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
    use super::{AreaTemplate, GridLine, LinePlacement, RepeatCount, TrackSize, parse_tracks};

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
    fn repeat_accepts_counts_and_auto_modes() {
        assert_eq!(
            parse_tracks("repeat(2, 100px 1fr)").unwrap(),
            [TrackSize::Repeat(
                RepeatCount::Count(2),
                vec![TrackSize::Px(100.0), TrackSize::Fr(1.0)]
            )]
        );
        assert_eq!(
            "repeat(auto-fill, 10px)".parse::<TrackSize>().unwrap(),
            TrackSize::Repeat(RepeatCount::AutoFill, vec![TrackSize::Px(10.0)])
        );
        assert!("repeat(0, 1fr)".parse::<TrackSize>().is_err());
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
    fn area_templates_form_rectangles() {
        let rows = ["head head".to_owned(), "side main".to_owned()];
        let template = AreaTemplate::parse(&rows).unwrap();
        assert_eq!((template.row_count(), template.column_count()), (2, 2));
        let areas = template.areas().unwrap();
        assert_eq!(areas[0].name, "head");
        assert_eq!(
            (
                areas[0].row_start,
                areas[0].row_end,
                areas[0].column_start,
                areas[0].column_end
            ),
            (1, 2, 1, 3)
        );
        assert_eq!(areas[1].name, "side");
        assert_eq!(
            (
                areas[1].row_start,
                areas[1].row_end,
                areas[1].column_start,
                areas[1].column_end
            ),
            (2, 3, 1, 2)
        );
        assert!(AreaTemplate::parse(&["a .".to_owned(), ". a".to_owned()]).is_err());
        assert!(AreaTemplate::parse(&["a".to_owned(), "b b".to_owned()]).is_err());
        assert!(AreaTemplate::parse(&["1a".to_owned()]).is_err());
    }

    #[test]
    fn malformed_grid_values_are_rejected() {
        assert!("1fr 2fr,".parse::<TrackSize>().is_err());
        assert!(parse_tracks("").is_err());
        assert!("2 / 3 / 4".parse::<GridLine>().is_err());
    }
}
