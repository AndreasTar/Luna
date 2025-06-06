
pub struct PositionInfo{
    pub minSize: [u16;2],
    pub maxSize: [u16;2],
    pub defaultSize: [u16;2],
    pub offset: [i16;2], 
    pub anchor: AnchorAt, 
    pub scaled: ScaledOn,
}

impl Default for PositionInfo {
    fn default() -> Self {
        Self { 
            minSize: [0;2], 
            maxSize: [u16::MAX;2], 
            defaultSize: [0;2], 
            offset: [0;2],
            anchor: AnchorAt::Center, 
            scaled: ScaledOn::Nothing,
        }
    }
}


/// Where to anchor the widget. 
/// 
/// This means that when the app gets resized, the anchor point of the widget will remain still, relative to the app window area.
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
/// the right axis, and half as fast downwards, relative to the amount of resizing on each axis accordingly.
/// If right wall increases by 4 pixels, the widget will increase by 8.
/// If bottom wall increases by 4 pixels, the widget will increase by 2.
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
    /// Scale this widget in all directions by `(x, y, z, w)` amount (percentile) respectively, `x` being right wall and going clockwise
    All(i8,i8,i8,i8)
}

// pub fn create_rectangle(ui: &Ui, posinfo: PositionInfo, debug: bool) -> Rect {

//     // TODO i need to add controls for 
//     //  - min height and width
//     //  - max height and width
//     //  - preferred height and width
//     //  

//     // i need
//     //      - size of area
//     //      - corners of area
//     //      - center of area
//     //      - corners of widget
//     let size = posinfo.defaultSize;
//     let anchor = posinfo.anchor;
//     let offset = posinfo.offset;

//     let uitl = ui.min_rect().min; // top left corner of available area
//     let uibr = ui.min_rect().max; // bottom right corner of available area
//     let uiSize = uibr - uitl; // size of available area
//     let center = Pos2::new(uiSize.x / 2.0, uiSize.y / 2.0); // center of available area
//     let halfsize = Vec2::new((size[0]/2).into(),(size[1]/2).into());
//     let fullsize = Vec2::new(size[0].into(), size[1].into());

//     if debug { // TODO change this to cfg debug
//         println!("{uiSize} {uitl} {uibr} {center} {halfsize} {fullsize}");
//     }

//     let res = match anchor {
//         AnchorAt::TopLeft =>        Rect::from_two_pos(Pos2::new(0.0, 0.0),                                     fullsize.to_pos2()),
//         AnchorAt::TopRight =>       Rect::from_two_pos(Pos2::new(uiSize.x - fullsize.x, 0.0),                   Pos2::new(uiSize.x, fullsize.y)),
//         AnchorAt::BottomLeft =>     Rect::from_two_pos(Pos2::new(0.0, uiSize.y - fullsize.y),                   Pos2::new(fullsize.x, uiSize.y)),
//         AnchorAt::BottomRight =>    Rect::from_two_pos((uiSize - fullsize).to_pos2(),                                uiSize.to_pos2()),
//         AnchorAt::TopCenter =>      Rect::from_two_pos(Pos2::new(center.x - halfsize.x, 0.0),                   Pos2::new(center.x + halfsize.x, fullsize.y)),
//         AnchorAt::BottomCenter =>   Rect::from_two_pos(Pos2::new(center.x - halfsize.x, uiSize.y - fullsize.y), Pos2::new(center.x + halfsize.x, uiSize.y)),
//         AnchorAt::CenterLeft =>     Rect::from_two_pos(Pos2::new(0.0, center.y - halfsize.y),                   Pos2::new(fullsize.x, center.y + halfsize.y)),
//         AnchorAt::CenterRight =>    Rect::from_two_pos(Pos2::new(uiSize.x - fullsize.x, center.y - halfsize.y), Pos2::new(uiSize.x, center.y + halfsize.y)),
//         AnchorAt::Center =>         Rect::from_two_pos(center - halfsize,                                            center + halfsize),
//     };

//     return res.translate(uitl.to_vec2() + Vec2::new(offset[0].into(), offset[1].into()));
// } 
