use paste::paste;

use super::ansi::ColorSequence;

macro_rules! create_enum_variants {
    ($vis:vis enum $name:ident ($prop:ident: $type:tt) { $($x:ident),+$(,)? }) => {
        paste! {
            #[derive(Debug, Clone, Copy)]
            $vis enum $name {
                $($x(#[doc = "`" $prop "`"] $type),)+
            }

            impl $name {
                #[doc = "Get `" $prop "`. (created from `create_enum_variants!`)"]
                $vis const fn [<$prop>](&self) -> &$type {
                    match self {
                        $($name::$x(data)=>data,)+
                    }
                }
                #[doc = "Set `" $prop "`. (created from `create_enum_variants!`)"]
                $vis fn [<set_ $prop>](&mut self, $prop: $type) {
                    match self {
                        $($name::$x(data)=>*data=$prop,)+
                    }
                }
            }
        }
    };
}

macro_rules! enum_values {
    ($vis:vis enum $name:ident = $type:tt { $($path:ident = $value:tt),+$(,)? }) => {
        impl $name {
            #[doc = "Get value corresponding the enum variant."]
            $vis const fn get_value(&self) -> $type {
                match self {
                    $($name::$path(..) => $value,)+
                }
            }
        }
    };
}

create_enum_variants!(
    pub enum Color4(is_bright: bool) {
        Black,
        Red,
        Green,
        Yellow,
        Blue,
        Magenta,
        Cyan,
        White
    }
);

enum_values!(
    pub enum Color4 = u8 {
        Black = 0,
        Red = 1,
        Green = 2,
        Yellow = 3,
        Blue = 4,
        Magenta = 5,
        Cyan = 6,
        White = 7
    }
);

impl ColorSequence for Color4 {
    fn get_sequence(&self, background: bool) -> String {
        if background {
            ((if *self.is_bright() { 100 } else { 40 }) + self.get_value()).to_string()
        } else {
            ((if *self.is_bright() { 90 } else { 30 }) + self.get_value()).to_string()
        }
    }
}
