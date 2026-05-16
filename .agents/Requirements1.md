## New App requirements

Create a new CLI application written in Rust that will be called `cep`, an acronym for ComEd Electricity Price, for fetching the current hourly rate as well as past data.

The CLI will have the following configuration options that can either be set in a command line flag using the `-` notation, in all lower case, or by setting an environment variable, all uppercase with underscores. The table will list the information required.

Option |  Short Option | Env Var Name | Description | Default Value
-------| --------------| ------------|--------------
base-url | b | BASE_URL | Base URL for the API call | `https://hourlypricing.comed.com`
type   | t | TYPE | Price type to get from API, either current `cur` or an internal range `range` | `current`
start | s | START | Start Date timestamp, used only when `type=range`, format YYYYMMddHHmm | current time - 24 hours
end | e | END | End Date timestamp, used only when `type=rng`, format YYYYMMddHHmm | current time
average | a | AVERAGE | Flag to indicate if a list or the average of the time period should be returned, possible values `y` and `n` - when set to n, it returns a list broken down by parameter time frame given in the option: `interval`  | `y`
interval | i | INTERVAL | Interval used only when `type=range`, supports format like 5m, 2h, 1d. Only minute (m), hour (h), and day (d) interavls area allowed. Min value=5m, max value=7d, this value will dicate how many response are returned in the list. for example if a user passes a start and end time that equates to 30 days, and the interval is set to 1d, then 30 values would be returned in the response| `1d`
format | f | FORMAT | Response formation, options: plain text (`text`), json (`json`), YAML (`yaml`), CSV (`csv`) | `json`

The CLI will call the API documented here to fetch the data required: https://hourlypricing.comed.com/hp-api/

Keep in mind that calculations will likely need to be performed, unless the user requests the 5m `interval`, since all responses in the list returned from the API will be in 5 minute intervals.  When the `interval` is set to anything other than the default `5m` then the values between the time periods will get averaged together into a single value.

## Requirements

* If the user passes a value for the `interval` that is larger than the time frame between `start` and `end` then just
* If the user passes an invalid option return error message indicating so and list the help contents
* If the user passes an invalid option value, return an error message indicating the possible values
* If the user passes a start date that is equal to or after the end date, provide an error message indicating so
* If the user passes an invalid date, provide an error message indicating so (remember it is optional field)
* Create a `-help` option that prints all of the config option values and description
