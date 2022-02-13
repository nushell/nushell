---
title: transpose
layout: command
version: 0.59.0
---

Transposes the table contents so rows become columns and columns become rows.

## Signature

transpose ...rest --header-row --ignore-titles

## Parameters

  ...rest: the names to give columns once transposed
  --header-row: treat the first row as column names
  --ignore-titles: don't transpose the column names into values

