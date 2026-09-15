let items = [10 20 30]
$items | each { |elt|
    let doubled = $elt * 2
    $doubled
}
