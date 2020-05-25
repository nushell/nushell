use crate::prelude::*;
use chrono::{Datelike, Local, NaiveDate};
use nu_errors::ShellError;
use nu_protocol::Dictionary;

use crate::commands::{command::EvaluatedWholeStreamCommandArgs, WholeStreamCommand};
use indexmap::IndexMap;
use nu_protocol::{ReturnSuccess, Signature, SyntaxShape, UntaggedValue, Value};

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

    fn examples(&self) -> Vec<Example> {
        vec![
            Example {
                description: "This month's calendar",
                example: "cal",
                result: None,
            },
            Example {
                description: "The calendar for all of 2012",
                example: "cal --full-year 2012",
                result: None,
            },
        ]
    }
}

pub fn cal(args: CommandArgs, registry: &CommandRegistry) -> Result<OutputStream, ShellError> {
    let registry = registry.clone();
    let stream = async_stream! {
        let args = args.evaluate_once(&registry).await?;
        let mut calendar_vec_deque = VecDeque::new();
        let tag = args.call_info.name_tag.clone();

        let (current_year, current_month, current_day) = get_current_date();

        let mut selected_year: i32 = current_year;
        let mut current_day_option: Option<u32> = Some(current_day);

        let month_range = if args.has("full-year") {
            if let Some(full_year_value) = args.get("full-year") {
                if let Ok(year_u64) = full_year_value.as_u64() {
                    selected_year = year_u64 as i32;

                    if selected_year != current_year {
                        current_day_option = None
                    }
                } else {
                    yield Err(get_invalid_year_shell_error(&full_year_value.tag()));
                    return;
                }
            }

            (1, 12)
        } else {
            (current_month, current_month)
        };

        let add_months_of_year_to_table_result = add_months_of_year_to_table(
            &args,
            &mut calendar_vec_deque,
            &tag,
            selected_year,
            month_range,
            current_month,
            current_day_option,
        );

        match add_months_of_year_to_table_result {
            Ok(()) => {
                for item in calendar_vec_deque {
                    yield ReturnSuccess::value(item);
                }
            }
            Err(error) => yield Err(error),
        }
    };

    Ok(stream.to_output_stream())
}

fn get_invalid_year_shell_error(year_tag: &Tag) -> ShellError {
    ShellError::labeled_error("The year is invalid", "invalid year", year_tag)
}

struct MonthHelper {
    day_number_month_starts_on: u32,
    number_of_days_in_month: u32,
    selected_year: i32,
    selected_month: u32,
}

impl MonthHelper {
    pub fn new(selected_year: i32, selected_month: u32) -> Result<MonthHelper, ()> {
        let mut month_helper = MonthHelper {
            day_number_month_starts_on: 0,
            number_of_days_in_month: 0,
            selected_year,
            selected_month,
        };

        let chosen_date_result_one = month_helper.update_day_number_month_starts_on();
        let chosen_date_result_two = month_helper.update_number_of_days_in_month();

        if chosen_date_result_one.is_ok() && chosen_date_result_two.is_ok() {
            return Ok(month_helper);
        }

        Err(())
    }

    pub fn get_month_name(&self) -> String {
        let month_name = match self.selected_month {
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

    fn update_day_number_month_starts_on(&mut self) -> Result<(), ()> {
        let naive_date_result =
            MonthHelper::get_naive_date(self.selected_year, self.selected_month);

        match naive_date_result {
            Ok(naive_date) => {
                self.day_number_month_starts_on = naive_date.weekday().num_days_from_sunday();
                Ok(())
            }
            _ => Err(()),
        }
    }

    fn update_number_of_days_in_month(&mut self) -> Result<(), ()> {
        // Chrono does not provide a method to output the amount of days in a month
        // This is a workaround taken from the example code from the Chrono docs here:
        // https://docs.rs/chrono/0.3.0/chrono/naive/date/struct.NaiveDate.html#example-30
        let (adjusted_year, adjusted_month) = if self.selected_month == 12 {
            (self.selected_year + 1, 1)
        } else {
            (self.selected_year, self.selected_month + 1)
        };

        let naive_date_result = MonthHelper::get_naive_date(adjusted_year, adjusted_month);

        match naive_date_result {
            Ok(naive_date) => {
                self.number_of_days_in_month = naive_date.pred().day();
                Ok(())
            }
            _ => Err(()),
        }
    }

    fn get_naive_date(selected_year: i32, selected_month: u32) -> Result<NaiveDate, ()> {
        if let Some(naive_date) = NaiveDate::from_ymd_opt(selected_year, selected_month, 1) {
            return Ok(naive_date);
        }

        Err(())
    }
}

fn get_current_date() -> (i32, u32, u32) {
    let local_now_date = Local::now().date();

    let current_year: i32 = local_now_date.year();
    let current_month: u32 = local_now_date.month();
    let current_day: u32 = local_now_date.day();

    (current_year, current_month, current_day)
}

fn add_months_of_year_to_table(
    args: &EvaluatedWholeStreamCommandArgs,
    mut calendar_vec_deque: &mut VecDeque<Value>,
    tag: &Tag,
    selected_year: i32,
    (start_month, end_month): (u32, u32),
    current_month: u32,
    current_day_option: Option<u32>,
) -> Result<(), ShellError> {
    for month_number in start_month..=end_month {
        let mut new_current_day_option: Option<u32> = None;

        if let Some(current_day) = current_day_option {
            if month_number == current_month {
                new_current_day_option = Some(current_day)
            }
        }

        let add_month_to_table_result = add_month_to_table(
            &args,
            &mut calendar_vec_deque,
            &tag,
            selected_year,
            month_number,
            new_current_day_option,
        );

        add_month_to_table_result?
    }

    Ok(())
}

fn add_month_to_table(
    args: &EvaluatedWholeStreamCommandArgs,
    calendar_vec_deque: &mut VecDeque<Value>,
    tag: &Tag,
    selected_year: i32,
    current_month: u32,
    _current_day_option: Option<u32>, // Can be used in the future to display current day
) -> Result<(), ShellError> {
    let month_helper_result = MonthHelper::new(selected_year, current_month);

    let month_helper = match month_helper_result {
        Ok(month_helper) => month_helper,
        Err(()) => match args.get("full-year") {
            Some(full_year_value) => {
                return Err(get_invalid_year_shell_error(&full_year_value.tag()))
            }
            None => {
                return Err(ShellError::labeled_error(
                    "Issue parsing command",
                    "invalid command",
                    tag,
                ))
            }
        },
    };

    let day_limit = month_helper.number_of_days_in_month + month_helper.day_number_month_starts_on;
    let mut day_count: u32 = 1;

    let days_of_the_week = [
        "sunday",
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
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
                UntaggedValue::int(((month_helper.selected_month - 1) / 3) + 1).into_value(tag),
            );
        }

        if should_show_month_column {
            let month_value = if should_show_month_names {
                UntaggedValue::string(month_helper.get_month_name()).into_value(tag)
            } else {
                UntaggedValue::int(month_helper.selected_month).into_value(tag)
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

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Cal;

    #[test]
    fn examples_work_as_expected() {
        use crate::examples::test as test_examples;

        test_examples(Cal {})
    }
}
