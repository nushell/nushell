use crate::prelude::*;
use chrono::{DateTime, Datelike, Local, NaiveDate};
use nu_errors::ShellError;
use nu_protocol::Dictionary;

use crate::commands::{command::EvaluatedWholeStreamCommandArgs, WholeStreamCommand};
use indexmap::IndexMap;
use nu_protocol::{Signature, SyntaxShape, UntaggedValue, Value};

pub struct Cal;

impl WholeStreamCommand for Cal {
    fn name(&self) -> &str {
        "cal"
    }

    fn signature(&self) -> Signature {
        Signature::build("cal")
            .switch("year", "Display the year column", Some('y'))
            .switch("quarter", "Display the quarter column", Some('q'))
            .switch("month", "Display the month column", Some('m'))
            .named(
                "full-year",
                SyntaxShape::Int,
                "Display a year-long calendar for the specified year",
                None,
            )
            .switch(
                "month-names",
                "Display the month names instead of integers",
                None,
            )
    }

    fn usage(&self) -> &str {
        "Display a calendar."
    }

    fn run(
        &self,
        args: CommandArgs,
        registry: &CommandRegistry,
    ) -> Result<OutputStream, ShellError> {
        cal(args, registry)
    }
}

pub fn cal(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let args = args.evaluate_once(registry)?;
    let mut calendar_vec_deque = VecDeque::new();
    let tag = args.call_info.name_tag.clone();

    let (current_year, current_month, current_day) = get_current_date();

    if args.has("full-year") {
        let mut selected_year: i32 = current_year;
        let mut current_day_option: Option<u32> = Some(current_day);

        if let Some(year_value) = args.get("full-year") {
            if let Ok(year_u64) = year_value.as_u64() {
                selected_year = year_u64 as i32;

                if selected_year != current_year {
                    current_day_option = None
                }
            }
        }

        let (month_helper, _) = MonthHelper::new(selected_year, current_year, current_month);

        add_year_to_table(
            &args,
            &mut calendar_vec_deque,
            &tag,
            &month_helper,
            current_day_option,
        );
    } else {
        let (month_helper, _) = MonthHelper::new(current_year, current_year, current_month);

        add_month_to_table(
            &args,
            &mut calendar_vec_deque,
            &tag,
            &month_helper,
            Some(current_day),
        );
    }

    Ok(futures::stream::iter(calendar_vec_deque).to_output_stream())
}

struct MonthHelper {
    day_number_month_starts_on: usize,
    number_of_days_in_month: usize,
    selected_year: i32,
    default_year_if_error: i32,
    month: u32,
}

impl MonthHelper {
    pub fn new(selected_year: i32, default_year_if_error: i32, month: u32) -> (MonthHelper, bool) {
        let mut month_helper = MonthHelper {
            day_number_month_starts_on: 0,
            number_of_days_in_month: 0,
            selected_year,
            default_year_if_error,
            month,
        };

        let chosen_date_is_valid_one = month_helper.calculate_day_number_month_starts_on();
        let chosen_date_is_valid_two = month_helper.calculate_number_of_days_in_month();

        (
            month_helper,
            chosen_date_is_valid_one && chosen_date_is_valid_two,
        )
    }

    pub fn get_month_name(&self) -> String {
        let month_name = match self.month {
            1 => "january",
            2 => "february",
            3 => "march",
            4 => "april",
            5 => "may",
            6 => "june",
            7 => "july",
            8 => "august",
            9 => "september",
            10 => "october",
            11 => "november",
            _ => "december",
        };

        month_name.to_string()
    }

    fn calculate_day_number_month_starts_on(&mut self) -> bool {
        let (naive_date, chosen_date_is_valid) =
            self.get_safe_naive_date(self.selected_year, self.month);
        let weekday = naive_date.weekday();

        self.day_number_month_starts_on = weekday.num_days_from_sunday() as usize;

        chosen_date_is_valid
    }

    fn calculate_number_of_days_in_month(&mut self) -> bool {
        // Chrono does not provide a method to output the amount of days in a month
        // This is a workaround taken from the example code from the Chrono docs here:
        // https://docs.rs/chrono/0.3.0/chrono/naive/date/struct.NaiveDate.html#example-30
        let (adjusted_year, adjusted_month) = if self.month == 12 {
            (self.selected_year + 1, 1)
        } else {
            (self.selected_year, self.month + 1)
        };

        let (naive_date, chosen_date_is_valid) =
            self.get_safe_naive_date(adjusted_year, adjusted_month);

        self.number_of_days_in_month = naive_date.pred().day() as usize;

        chosen_date_is_valid
    }

    fn get_safe_naive_date(&self, selected_year: i32, selected_month: u32) -> (NaiveDate, bool) {
        if let Some(naive_date) = NaiveDate::from_ymd_opt(selected_year, selected_month, 1) {
            return (naive_date, true);
        }

        (
            NaiveDate::from_ymd(self.default_year_if_error, selected_month, 1),
            false,
        )
    }
}

fn get_current_date() -> (i32, u32, u32) {
    let local_now: DateTime<Local> = Local::now();

    let current_year: i32 = local_now.date().year();
    let current_month: u32 = local_now.date().month();
    let current_day: u32 = local_now.date().day();

    (current_year, current_month, current_day)
}

fn add_year_to_table(
    args: &EvaluatedWholeStreamCommandArgs,
    mut calendar_vec_deque: &mut VecDeque<Value>,
    tag: &Tag,
    current_month_helper: &MonthHelper,
    current_day_option: Option<u32>,
) {
    for month_number in 1..=12 {
        let (mut month_helper, chosen_date_is_valid) = MonthHelper::new(
            current_month_helper.selected_year,
            current_month_helper.default_year_if_error,
            month_number,
        );

        if !chosen_date_is_valid {
            month_helper.selected_year = month_helper.default_year_if_error;
        }

        let mut new_current_day_option: Option<u32> = None;

        if let Some(current_day) = current_day_option {
            if month_number == month_helper.month {
                new_current_day_option = Some(current_day)
            }
        }

        add_month_to_table(
            &args,
            &mut calendar_vec_deque,
            &tag,
            &month_helper,
            new_current_day_option,
        );
    }
}

fn add_month_to_table(
    args: &EvaluatedWholeStreamCommandArgs,
    calendar_vec_deque: &mut VecDeque<Value>,
    tag: &Tag,
    month_helper: &MonthHelper,
    _current_day_option: Option<u32>, // Can be used in the future to display current day
) {
    let day_limit = month_helper.number_of_days_in_month + month_helper.day_number_month_starts_on;
    let mut day_count: usize = 1;

    let days_of_the_week = [
        "sunday",
        "monday",
        "tuesday",
        "wednesday",
        "thurday",
        "friday",
        "saturday",
    ];

    let should_show_year_column = args.has("year");
    let should_show_month_column = args.has("month");
    let should_show_quarter_column = args.has("quarter");
    let should_show_month_names = args.has("month-names");

    while day_count <= day_limit {
        let mut indexmap = IndexMap::new();

        if should_show_year_column {
            indexmap.insert(
                "year".to_string(),
                UntaggedValue::int(month_helper.selected_year).into_value(tag),
            );
        }

        if should_show_quarter_column {
            indexmap.insert(
                "quarter".to_string(),
                UntaggedValue::int(((month_helper.month - 1) / 3) + 1).into_value(tag),
            );
        }

        if should_show_month_column {
            let month_value = if should_show_month_names {
                UntaggedValue::string(month_helper.get_month_name()).into_value(tag)
            } else {
                UntaggedValue::int(month_helper.month).into_value(tag)
            };

            indexmap.insert("month".to_string(), month_value);
        }

        for day in &days_of_the_week {
            let value = if (day_count <= day_limit)
                && (day_count > month_helper.day_number_month_starts_on)
            {
                UntaggedValue::int(day_count - month_helper.day_number_month_starts_on)
                    .into_value(tag)
            } else {
                UntaggedValue::nothing().into_value(tag)
            };

            indexmap.insert((*day).to_string(), value);

            day_count += 1;
        }

        calendar_vec_deque
            .push_back(UntaggedValue::Row(Dictionary::from(indexmap)).into_value(tag));
    }
}
