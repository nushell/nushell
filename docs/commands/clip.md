# clip (Windows Only)

use clip to redirect the output of a command to windows clipboard (copy)


# Syntax
```cmd
<command> | clip
```
```cmd
clip < <file to copy from>
```

# Parameters

|Parameter     | Description                                    |
|--------------|------------------------------------------------|
| \<Command>   | command whose output needs to be copied        |
|  \<FileName> | file whose content needs to be copied          |
| /?           | Display help

## Examples
1. To copy the current directory list:
```cmd
dir | clip
```
2. To copy the contents of a file called README.md
```cmd
clip < README.md
```