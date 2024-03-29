% fancy(1) | Fancy CLI

# NAME

fancy - Fancy CLI

# SYNOPSIS

`fancy get [speeds | temps | config | auto | status]`

`fancy set [-f FAN_SPEED [FAN_SPEEDS ...] | -a] [-c CONFIGURATION]`

`fancy list [--recommended]`

# DESCRIPTION

fancy is the CLI of _fancy(7)_,
a set of software which allows to control laptop fans.

# OPTIONS

#### SET

`-f, --fans-speeds FAN_SPEEDS...`

: Set fans speeds by percentage, between 0 and 100

`-c, --config CONFIGURATION`

: Set the configuration used by the daemon

`-a, --auto`
: Let the daemon automatically choose the speed, according to the temperature

`-m, --manual`
: Apply the defined target speeds while the temperature is not critical
(meaning when the computer starts to burn your legs).

#### GET

`fancy get speeds`

: Get fans speeds

`fancy get target-speeds`

: Get fans target speeds

`fancy get temps`

: Get temperatures

`fancy get config`

: Get current configuration

`fancy get auto`

: Get automatic speed management state

`fancy get status`

: Get summary

#### LIST

List all available configurations

`--recommended`

: List only recommended configurations

# BUGS

Bugs can be reported at https://github.com/MusiKid/fancy
