use std.nu

export def test_xml_basic [] {
    use std.nu "xaccess"
    use std.nu "xupdate"
    use std.nu "assert equal"

    let sample_xml = ('<a><b><c a="b"></c></b><c></c><d><e>z</e><e>x</e></d></a>' | from xml)

    assert equal ($sample_xml | xaccess [a]) [$sample_xml]
    assert equal ($sample_xml | xaccess [*]) [$sample_xml]
    assert equal ($sample_xml | xaccess [* d e]) [[tag, attributes, content]; [e, {}, [[tag, attributes, content]; [null, null, z]]], [e, {}, [[tag, attributes, content]; [null, null, x]]]]
    assert equal ($sample_xml | xaccess [* d e 1]) [[tag, attributes, content]; [e, {}, [[tag, attributes, content]; [null, null, x]]]]
    assert equal ($sample_xml | xupdate [*] {|x| $x | update attributes {i: j}}) ('<a i="j"><b><c a="b"></c></b><c></c><d><e>z</e><e>x</e></d></a>' | from xml)
    assert equal ($sample_xml | xupdate [* d e *] {|x| $x | update content 'nushell'}) ('<a><b><c a="b"></c></b><c></c><d><e>nushell</e><e>nushell</e></d></a>' | from xml)
    
}