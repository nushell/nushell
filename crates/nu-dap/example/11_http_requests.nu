# HTTP requests: http get/post, JSON parsing, API interaction.
# Demonstrates real network calls — inspect the response objects in the
# Variables panel to explore their structure.

# Simple GET — returns parsed JSON automatically
print "--- Fetching a TODO item ---"
let todo = http get "https://jsonplaceholder.typicode.com/todos/1"
print $todo
print $"Title: ($todo.title)"
print $"Completed: ($todo.completed)"

# GET a list and filter it
print "\n--- Fetching users ---"
let users = http get "https://jsonplaceholder.typicode.com/users"
let names = $users | select name email
print ($names | first 3)

# GET with query parameters
print "\n--- Fetching posts by user 1 ---"
let posts = http get "https://jsonplaceholder.typicode.com/posts?userId=1"
let titles = $posts | get title | first 3
print $titles

# POST request with a JSON body.
# `http post` takes the body as a string/binary, so build the record and
# serialize it with `to json` (set --content-type so the API parses it).
print "\n--- Creating a new post ---"
let body = {
    title: "Debug Session Notes"
    body: "Stepping through HTTP calls is great for understanding APIs."
    userId: 1
}
let new_post = http post --content-type application/json "https://jsonplaceholder.typicode.com/posts" ($body | to json)
print $"Created post id: ($new_post.id)"

# Working with the response — extract and transform
print "\n--- Comment analysis ---"
let comments = http get "https://jsonplaceholder.typicode.com/posts/1/comments"
let summary = $comments | each { |c|
    {
        author: $c.name
        email: $c.email
        length: ($c.body | str length)
    }
}
print $summary

# Error handling for HTTP requests
print "\n--- Handling HTTP errors ---"
try {
    let bad = http get "https://jsonplaceholder.typicode.com/nonexistent"
    print $bad
} catch { |err|
    print $"Request failed: ($err.msg)"
}
