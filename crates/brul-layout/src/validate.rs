use crate::{Dimension, GridLine, LayoutError, LayoutStyle, LinePlacement};

pub fn validate(style: &LayoutStyle) -> Result<(), LayoutError> {
    validate_dimension(style.width)?;
    validate_dimension(style.height)?;
    validate_flex(style)?;
    validate_grid_line(&style.grid_column)?;
    validate_grid_line(&style.grid_row)
}

fn valid_placement(placement: LinePlacement) -> bool {
    match placement {
        LinePlacement::Auto => true,
        LinePlacement::Line(value) => i16::try_from(value).is_ok_and(|line| line != 0),
        LinePlacement::Span(value) => value > 0,
    }
}

fn validate_grid_line(line: &GridLine) -> Result<(), LayoutError> {
    let mut placements = std::iter::once(line.start).chain(line.end);
    if placements.all(valid_placement) {
        Ok(())
    } else {
        Err(LayoutError::InvalidGridLine)
    }
}

fn validate_dimension(dimension: Dimension) -> Result<(), LayoutError> {
    if let Dimension::Percent(percent) = dimension
        && (!percent.is_finite() || percent < 0.0)
    {
        return Err(LayoutError::InvalidDimension);
    }
    Ok(())
}

fn validate_flex(style: &LayoutStyle) -> Result<(), LayoutError> {
    let factors = [style.flex_grow, style.flex_shrink];
    if factors
        .iter()
        .any(|factor| !factor.is_finite() || *factor < 0.0)
    {
        return Err(LayoutError::InvalidFlex);
    }
    if let Dimension::Percent(percent) = style.flex_basis
        && (!percent.is_finite() || percent < 0.0)
    {
        return Err(LayoutError::InvalidFlex);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
