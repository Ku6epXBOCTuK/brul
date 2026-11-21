use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AreaTemplateError;

impl fmt::Display for AreaTemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid area template")
    }
}

impl std::error::Error for AreaTemplateError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepeatCount {
    Count(u16),
    AutoFill,
    AutoFit,
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
    pub fn parse(rows: &[String]) -> Result<Self, AreaTemplateError> {
        let parsed = rows
            .iter()
            .map(|row| {
                row.split_whitespace()
                    .map(|cell| match cell {
                        "." => Ok(None),
                        name if valid_area_name(name) => Ok(Some(name.to_owned())),
                        _ => Err(AreaTemplateError),
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        let width = parsed.first().ok_or(AreaTemplateError)?.len();
        if width == 0
            || parsed.len() > u16::MAX as usize
            || width > u16::MAX as usize
            || parsed.iter().any(|row| row.len() != width)
        {
            return Err(AreaTemplateError);
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

    pub fn areas(&self) -> Result<Vec<Area>, AreaTemplateError> {
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
                return Err(AreaTemplateError);
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LinePlacement {
    #[default]
    Auto,
    Line(i32),
    Span(u16),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GridLine {
    pub start: LinePlacement,
    pub end: Option<LinePlacement>,
}

#[cfg(test)]
mod tests {
    use super::AreaTemplate;

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
}
