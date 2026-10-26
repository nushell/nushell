{||
    if $in < 1hr {
      'red'
      } else if $in < 1wk {
      'green'
    } else if $in < 6wk {
      'blue'
    } else { 'gray' }
  }
