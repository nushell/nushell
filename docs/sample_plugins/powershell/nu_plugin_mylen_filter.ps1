#!/usr/bin/env pwsh

# Created to demonstrate how to create a plugin with PowerShell
# Below is a list of other links to help with scripting language plugin creation
# https://vsoch.github.io/2019/nushell-plugin-golang/
# Go        https://github.com/vsoch/nushell-plugin-len
# Python    https://github.com/vsoch/nushell-plugin-python
# Python    https://github.com/vsoch/nushell-plugin-pokemon
# C#        https://github.com/myty/nu-plugin-lib
# Ruby      https://github.com/andrasio/nu_plugin

# WIP 8/26/20
# This is not yet ready for prime time but it could give someone
# a headstart with writing powershell core plugins for mac/linux/win
# Most of the comments are TODO's or functions from python plugins
# meant to be used as a reference.
#
# While this length demonstration works, it is filled with debug
# messages that can be seen during normal operation.
#
# This is by no means complete. Good luck!

# def print_good_response(response):
#     json_response = {"jsonrpc": "2.0", "method": "response", "params": {"Ok": response}}
#     print(json.dumps(json_response))
#     sys.stdout.flush()

function print_good_response {
    param($response)
    $json_response = @"
{"jsonrpc": "2.0", "method": "response", "params": {"Ok": $($response)}}
"@
    Write-Host $json_response
}

# def get_length(string_value):
#     string_len = len(string_value["item"]["Primitive"]["String"])
#     int_item = {"Primitive": {"Int": string_len}}
#     int_value = string_value
#     int_value["item"] = int_item
#     return int_value

# function get_length {
#     param($string_val)
#     $string_len = $string_val[`"item`"][`"Primitive`"][`"String`"].Length
# }

function config {
    param ($json_rpc)
    #Write-Host $json_rpc

    $response = '{ "jsonrpc": "2.0", "method": "response", "params": { "Ok": { "name": "mylen", "usage": "Return the length of a string", "positional": [], "rest_positional": null, "named": {}, "is_filter": true } } }'
    Write-Host $response
    return
}

function begin_filter {
    $response = '{"jsonrpc":"2.0","method":"response","params":{"Ok":[]}}'
    Write-Host $response
    return
}

function run_filter {
    param($input_data)
    Write-TraceMessage "PIPED" $input_data

    $prim = "Primitive"
    $method = $input_data | Select-Object "method"
    $params = $input_data.params
    $primitive = $input_data.params.value.$prim
    $prim_type = ""
    $len = 0

    if (![String]::IsNullOrEmpty($input_data)) {
        Write-TraceMessage "FJSON" $input_data
    }
    if (![String]::IsNullOrEmpty($method)) {
        Write-TraceMessage "FMETHOD" $method
    }
    if (![String]::IsNullOrEmpty($params)) {
        Write-TraceMessage "FPARAMS" $params
    }
    if (![String]::IsNullOrEmpty($primitive)) {
        Write-TraceMessage "FPRIMITIVE" $primitive
        # $prim_type = $primitive | Get-Member -MemberType NoteProperty | Select-Object Name
        # switch ($prim_type.Name) {
        #     'String' { $data.params.value.$prim.String }
        #     'Int' { $data.params.value.$prim.Int }
        #     Default { "none-found" }
        # }
    }

    $prim_type = $primitive | Get-Member -MemberType NoteProperty | Select-Object Name
    switch ($prim_type.Name) {
        'String' { $len = $input_data.params.value.$prim.String.Length }
        'Int' { $input_data.params.value.$prim.Int }
        Default { $len = 0 }
    }

    # Create a json hash making sure all the child nodes are sorted
    $json_obj = [ordered]@{
        jsonrpc = "2.0"
        method  = "response"
        params  = [ordered]@{
            Ok = @(
                [ordered]@{
                    Ok = [ordered]@{
                        Value = [ordered]@{
                            value = [ordered]@{
                                Primitive = [ordered]@{
                                    Int = $len
                                }
                            }
                            tag   = [ordered]@{
                                anchor = $null
                                span   = @{
                                    end   = 2
                                    start = 0
                                }
                            }
                        }
                    }
                }
            )
        }
    }
    $response = $json_obj | ConvertTo-Json -Depth 100 -Compress
    Write-Host $response

    return
}

function end_filter {
    $response = '{"jsonrpc":"2.0","method":"response","params":{"Ok":[]}}'
    Write-Host $response
    return
}

function Write-TraceMessage {
    Param
    (
        [Parameter(Mandatory = $false, Position = 0)]
        [string] $label,
        [Parameter(Mandatory = $false, Position = 1)]
        [string] $message
    )

    [Console]::Error.WriteLine("$($label) $($message)")
}

function run_loop {
    param($data)
    $prim = "Primitive"
    $method = $data | Select-Object "method"
    $params = $data.params
    $primitive = $data.params.value.$prim
    # $prim_type = ""
    # Write out some debug trace messages
    if (![String]::IsNullOrEmpty($data)) {
        Write-TraceMessage "JSON" $data
    }
    if (![String]::IsNullOrEmpty($method)) {
        Write-TraceMessage "METHOD" $method
    }
    if (![String]::IsNullOrEmpty($params)) {
        Write-TraceMessage "PARAMS" $params
    }
    if (![String]::IsNullOrEmpty($primitive)) {
        Write-TraceMessage "PRIMITIVE" $primitive
        # $prim_type = $primitive | Get-Member -MemberType NoteProperty | Select-Object Name
        # switch ($prim_type.Name) {
        #     'String' { $data.params.value.$prim.String }
        #     'Int' { $data.params.value.$prim.Int }
        #     Default { "none-found" }
        # }
    }


    if ($method[0].method -eq "config") {
        # Write-TraceMessage "Received config method with: " $data
        return config
    }
    elseif ($method[0].method -eq "begin_filter") {
        return begin_filter
    }
    elseif ($method[0].method -eq "end_filter") {
        return end_filter
    }
    elseif ($method[0].method -eq "filter") {
        # return run_filter -piped $params
        return run_filter -input_data $data
    }
}

function Get-PipedData {
    param(
        [Parameter(
            Position = 0,
            Mandatory = $true,
            ValueFromPipeline = $true,
            ValueFromPipelineByPropertyName = $true)
        ]
        [Alias('piped')]
        [String]$piped_input
    )

    process {
        # Write-Error $piped_input
        Write-TraceMessage "BeforeJSON" $piped_input
        $json = ConvertFrom-Json $piped_input
        run_loop -data $json
    }
}

# $prim = "Primitive"
# $j = $json | ConvertFrom-Json
# $j.params.value.$prim
# String
# ------
# 123

# $Input | Get-PipedData
$Input | ForEach-Object { $_ | Get-PipedData }