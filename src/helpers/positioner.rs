use egui::{Rect, Ui, Widget};

// NOTE i could do these like slint has, with a Rectangle{} struct that contains params like preffered-width and horizontal-stretch


/// Where to anchor the widget. 
/// 
/// This means that when the app gets resized, the anchor point of the widget will remain still relative to the app window.
pub enum AnchorAt {
    /// Equivalent to `VerticalAlign::Top` + `HorizontalAlign::Left`
    /// aka this point:
    /// ```
    /// +-------+
    /// | o . . |
    /// | . . . |
    /// | . . . |
    /// +-------+
    /// ```
    TopLeft,
    /// Equivalent to `VerticalAlign::Top` + `HorizontalAlign::Right`
    /// aka this point:
    /// ```
    /// +-------+
    /// | . . o |
    /// | . . . |
    /// | . . . |
    /// +-------+
    /// ```
    TopRight,
    /// Equivalent to `VerticalAlign::Bot` + `HorizontalAlign::Left`
    /// aka this point:
    /// ```
    /// +-------+
    /// | . . . |
    /// | . . . |
    /// | o . . |
    /// +-------+
    /// ```
    BottomLeft,
    /// Equivalent to `VerticalAlign::Bot` + `HorizontalAlign::Right`
    /// aka this point:
    /// ```
    /// +-------+
    /// | . . . |
    /// | . . . |
    /// | . . o |
    /// +-------+
    /// ```
    BottomRight,
    /// Equivalent to `VerticalAlign::Top` + `HorizontalAlign::Center`
    /// aka this point:
    /// ```
    /// +-------+
    /// | . o . |
    /// | . . . |
    /// | . . . |
    /// +-------+
    /// ```
    TopCenter,
    /// Equivalent to `VerticalAlign::Bot` + `HorizontalAlign::Center`
    /// aka this point:
    /// ```
    /// +-------+
    /// | . . . |
    /// | . . . |
    /// | . o . |
    /// +-------+
    /// ```
    BottomCenter,
    /// Equivalent to `VerticalAlign::Center` + `HorizontalAlign::Left`
    /// aka this point:
    /// ```
    /// +-------+
    /// | . . . |
    /// | o . . |
    /// | . . . |
    /// +-------+
    /// ```
    CenterLeft,
    /// Equivalent to `VerticalAlign::Center` + `HorizontalAlign::Right`
    /// aka this point:
    /// ```
    /// +-------+
    /// | . . . |
    /// | . . o |
    /// | . . . |
    /// +-------+
    /// ```
    CenterRight,
    /// Equivalent to `VerticalAlign::Center` + `HorizontalAlign::Center`
    /// aka this point:
    /// ```
    /// +-------+
    /// | . . . |
    /// | . o . |
    /// | . . . |
    /// +-------+
    /// ```
    Center
}


/// What axis should the widget be scaled, when the app window gets resized OR the UI area changes
/// 
/// Numbers are percentile, meaning a value of `ScaledOn::RightDown(2, 0.5)` will scale the widget twice as fast on
/// the right axis, and half as fast downwards, relative to the amount of resizing on each axis accordingly
pub enum ScaledOn {
    /// Dont scale this widget at all; keep the dimensions constant
    Nothing,
    /// Scale this widget only towards the Right by `x` amount (percentile)
    Right(i8),
    /// Scale this widget only towards the Left by `x` amount (percentile)
    Left(i8),
    /// Scale this widget only Upwards by `y` amount (percentile)
    Up(i8),
    /// Scale this widget only Downwards by `y` amount (percentile)
    Down(i8),
    /// Scale this widget diagonally towards Bottom-Right corner by `(x, y)` amount (percentile)
    RightDown(i8, i8),
    /// Scale this widget diagonally towards Top-Right corner by `(x, y)` amount (percentile)
    RightUp(i8, i8),
    /// Scale this widget diagonally towards Bottom-Left corner by `(x, y)` amount (percentile)
    LeftDown(i8, i8),
    /// Scale this widget diagonally towards Top-Left corner by `(x, y)` amount (percentile)
    LeftUp(i8, i8),
    /// Scale this widget in all directions by `(x, y, z, w)` amount (percentile) respectively
    All(i8,i8,i8,i8)
}

pub fn create_rectangle(ui: &Ui, wid: impl Widget, anchor: AnchorAt, scaled: ScaledOn) -> Rect {

    // TODO i need to add controls for 
    //  - min height and width
    //  - max height and width
    //  - preferred height and width
    //  

    match anchor {
        AnchorAt::TopLeft => todo!(), //Rect::from_min_size(ui.min_rect().min, ),
        AnchorAt::TopRight => todo!(),
        AnchorAt::BottomLeft => todo!(),
        AnchorAt::BottomRight => todo!(),
        AnchorAt::TopCenter => todo!(),
        AnchorAt::BottomCenter => todo!(),
        AnchorAt::CenterLeft => todo!(),
        AnchorAt::CenterRight => todo!(),
        AnchorAt::Center => todo!(),
    };
    

    todo!()
}