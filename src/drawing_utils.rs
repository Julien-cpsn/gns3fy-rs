//! Helpers to build SVG strings for project drawings.

/// Options for [`generate_rectangle_svg`].
#[derive(Debug, Clone, PartialEq)]
pub struct RectangleOptions {
    pub height: i64,
    pub width: i64,
    pub fill: String,
    pub fill_opacity: f64,
    pub stroke: String,
    pub stroke_width: i64,
}

impl Default for RectangleOptions {
    fn default() -> Self {
        Self {
            height: 100,
            width: 200,
            fill: "#ffffff".into(),
            fill_opacity: 1.0,
            stroke: "#000000".into(),
            stroke_width: 2,
        }
    }
}

/// Options for [`generate_ellipse_svg`].
#[derive(Debug, Clone, PartialEq)]
pub struct EllipseOptions {
    pub height: f64,
    pub width: f64,
    pub cx: i64,
    pub cy: i64,
    pub fill: String,
    pub fill_opacity: f64,
    pub rx: i64,
    pub ry: i64,
    pub stroke: String,
    pub stroke_width: i64,
}

impl Default for EllipseOptions {
    fn default() -> Self {
        Self {
            height: 200.0,
            width: 200.0,
            cx: 100,
            cy: 100,
            fill: "#ffffff".into(),
            fill_opacity: 1.0,
            rx: 100,
            ry: 100,
            stroke: "#000000".into(),
            stroke_width: 2,
        }
    }
}

/// Options for [`generate_line_svg`].
#[derive(Debug, Clone, PartialEq)]
pub struct LineOptions {
    pub height: i64,
    pub width: i64,
    pub x1: i64,
    pub x2: i64,
    pub y1: i64,
    pub y2: i64,
    pub stroke: String,
    pub stroke_width: i64,
}

impl Default for LineOptions {
    fn default() -> Self {
        Self {
            height: 0,
            width: 200,
            x1: 0,
            x2: 200,
            y1: 0,
            y2: 0,
            stroke: "#000000".into(),
            stroke_width: 2,
        }
    }
}

// `{:?}` keeps a trailing `.0` on whole floats ("1.0"), matching Python's float formatting.

pub fn generate_rectangle_svg(o: &RectangleOptions) -> String {
    format!(
        r#"<svg height="{h}" width="{w}"><rect fill="{fill}" fill-opacity="{fo:?}" height="{h}" stroke="{stroke}" stroke-width="{sw}" width="{w}" /></svg>"#,
        h = o.height,
        w = o.width,
        fill = o.fill,
        fo = o.fill_opacity,
        stroke = o.stroke,
        sw = o.stroke_width,
    )
}

pub fn generate_ellipse_svg(o: &EllipseOptions) -> String {
    format!(
        r#"<svg height="{h:?}" width="{w:?}"><ellipse cx="{cx}" cy="{cy}" fill="{fill}" fill-opacity="{fo:?}" rx="{rx}" ry="{ry}" stroke="{stroke}" stroke-width="{sw}" /></svg>"#,
        h = o.height,
        w = o.width,
        cx = o.cx,
        cy = o.cy,
        fill = o.fill,
        fo = o.fill_opacity,
        rx = o.rx,
        ry = o.ry,
        stroke = o.stroke,
        sw = o.stroke_width,
    )
}

pub fn generate_line_svg(o: &LineOptions) -> String {
    format!(
        r#"<svg height="{h}" width="{w}"><line stroke="{stroke}" stroke-width="{sw}" x1="{x1}" x2="{x2}" y1="{y1}" y2="{y2}" /></svg>"#,
        h = o.height,
        w = o.width,
        stroke = o.stroke,
        sw = o.stroke_width,
        x1 = o.x1,
        x2 = o.x2,
        y1 = o.y1,
        y2 = o.y2,
    )
}

/// Converts a grid column to a GNS3 x coordinate (`obj_width` defaults to 100 in Python).
pub fn parsed_x(x: i64, obj_width: i64) -> i64 {
    x * obj_width
}

/// Converts a grid row to a GNS3 y coordinate (y grows downwards in GNS3).
pub fn parsed_y(y: i64, obj_height: i64) -> i64 {
    -(y * obj_height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle() {
        assert_eq!(
            generate_rectangle_svg(&RectangleOptions::default()),
            "<svg height=\"100\" width=\"200\"><rect fill=\"#ffffff\" fill-opacity=\"1.0\" \
             height=\"100\" stroke=\"#000000\" stroke-width=\"2\" width=\"200\" /></svg>"
        );
    }

    #[test]
    fn ellipse() {
        assert_eq!(
            generate_ellipse_svg(&EllipseOptions::default()),
            "<svg height=\"200.0\" width=\"200.0\"><ellipse cx=\"100\" cy=\"100\" fill=\"#ffffff\" \
             fill-opacity=\"1.0\" rx=\"100\" ry=\"100\" stroke=\"#000000\" stroke-width=\"2\" />\
             </svg>"
        );
    }

    #[test]
    fn line() {
        assert_eq!(
            generate_line_svg(&LineOptions::default()),
            "<svg height=\"0\" width=\"200\"><line stroke=\"#000000\" stroke-width=\"2\" x1=\"0\" \
             x2=\"200\" y1=\"0\" y2=\"0\" /></svg>"
        );
    }

    #[test]
    fn coordinates() {
        assert_eq!(parsed_x(7, 100), 700);
        assert_eq!(parsed_y(7, 100), -700);
    }
}
