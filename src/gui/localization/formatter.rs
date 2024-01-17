use fluent::FluentValue;
use fluent::memoizer::MemoizerKind;
use fluent::types::{FluentNumber, FluentNumberStyle};

fn number_formatter(num: &FluentNumber) -> Option<String> {
    match num.options.style {
        FluentNumberStyle::Decimal => {
            let i_min = num.options.minimum_integer_digits.unwrap_or(0);
            let f_min = num.options.minimum_fraction_digits.unwrap_or(0);
            let f_max = num.options.maximum_fraction_digits.unwrap_or(64);

            if num.value.fract() == 0.0 && f_min == 0 {
                return Some(format!("{:0>i_min$}", num.value));
            }

            let num_str = format!("{:?}", num.value);
            let (i_part, mut f_part) = num_str
                .split_once('.')
                .map(|(i, f)| (i.to_string(), f.to_string()))
                .unwrap();

            if f_part.len() > f_max {
                f_part.truncate(f_max + 1);
                f_part = (f_part.parse::<f64>().unwrap() / 10.0).round().to_string();
            }
            Some(format!("{:0>i_min$}.{:0<f_min$}", i_part, f_part))
        }
        FluentNumberStyle::Percent => Some(format!("{}%", (num.value * 100.0).round())),
        FluentNumberStyle::Currency => None
    }
}

pub fn fluent_formatter<M: MemoizerKind>(value: &FluentValue<'_>, _memoizer: &M) -> Option<String> {
    match value {
        FluentValue::Number(num) => number_formatter(num),
        _ => None
    }
}
