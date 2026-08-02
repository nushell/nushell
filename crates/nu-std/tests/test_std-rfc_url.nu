use std-rfc/url
use std/assert
use std/testing *

@test
def url_with_host [] {
    let new_url = 'https://www.nushell.sh/blog/' | url with-host 'www.google.com'
    assert equal $new_url 'https://www.google.com/blog/'
}

@test
def url_with_port [] {
    let new_url = 'http://localhost' | url with-port 8080
    assert equal $new_url 'http://localhost:8080/'
}

@test
def url_with_path [] {
    let new_url = 'https://www.nushell.sh/' | url with-path 'blog'
    assert equal $new_url 'https://www.nushell.sh/blog'
}

@test
def url_with_fragment [] {
    let new_url = 'https://www.nushell.sh/book/#this-book' | url with-fragment 'introduction'
    assert equal $new_url 'https://www.nushell.sh/book/#introduction'
}

@test
def url_with_params_record [] {
    let new_url = 'https://github.com/nushell/nushell/pulls?q=is%3Aopen' | url with-params {q: 'is:closed'}
    assert equal $new_url 'https://github.com/nushell/nushell/pulls?q=is%3Aclosed'
}

@test
def url_with_params_table [] {
    let new_url = 'https://github.com/nushell/nushell/pulls?q=is%3Aopen'
    | url with-params [[key, value]; ['q', 'is:closed']]

    assert equal $new_url 'https://github.com/nushell/nushell/pulls?q=is%3Aclosed'
}

@test
def url_replace_passthru [] {
    let new_url = 'https://www.nushell.sh/book' | url replace
    assert equal $new_url 'https://www.nushell.sh/book'
}

@test
def url_replace_single_field [] {
    let new_url = 'https://www.nushell.sh/book' | url replace --path 'book/nu_fundamentals.html'
    assert equal $new_url 'https://www.nushell.sh/book/nu_fundamentals.html'
}

@test
def url_replace_multiple_fields [] {
    let new_url = 'https://github.com/nushell/nushell' | url replace --path 'nushell/reedline/issues' --params {q: 'is:issue'}
    assert equal $new_url 'https://github.com/nushell/reedline/issues?q=is%3Aissue'
}
