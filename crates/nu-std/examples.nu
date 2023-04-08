use std 'log debug'
use std 'log info'
use std 'log warning'
use std 'log error'
use std 'log critical'

export def log [] {
    log debug "Debug message"
    log info "Info message"
    log warning "Warning message"
    log error "Error message"
    log critical "Critical message"
}
