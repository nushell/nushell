# start

Opens each file/directory/URL using the default application.

Syntax: `start ...args{flags}`

## Parameters

* `args`: a list of space-separated files to open

## Flags

    -a --application <string>
      Specifies the application used for opening the files/directories/urls

## Example

Open `index.html` in the system's default browser (cross platform):

```shell
> start index.html
```

Open `index.html` in Firefox (specific path for OSX):

```shell
start index.html -a /Applications/Firefox.app
```
