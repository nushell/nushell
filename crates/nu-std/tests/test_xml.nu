use std/testing *
use std/xml *
use std/assert

@before-each
def before-each [] {
    {sample_xml: ('
        <a>
            <b>
                <c a="b"></c>
            </b>
            <c></c>
            <d>
                <e>z</e>
                <e>x</e>
            </d>
        </a>' | from xml)
    }
}

@test
def xml_xaccess [] {
    let sample_xml = $in.sample_xml

    assert equal ($sample_xml | xaccess [a]) [$sample_xml]
    assert equal ($sample_xml | xaccess [*]) [$sample_xml]
    assert equal ($sample_xml | xaccess [* d e]) [[tag, attributes, content]; [e, {}, [[tag, attributes, content]; [null, null, z]]], [e, {}, [[tag, attributes, content]; [null, null, x]]]]
    assert equal ($sample_xml | xaccess [* d e 1]) [[tag, attributes, content]; [e, {}, [[tag, attributes, content]; [null, null, x]]]]
    assert equal ($sample_xml | xaccess [* * * {|e| $e.attributes != {}}]) [[tag, attributes, content]; [c, {a: b}, []]]
}

@test
def xml_xupdate [] {
    let sample_xml = $in.sample_xml

    assert equal ($sample_xml | xupdate [*] {|x| $x | update attributes {i: j}}) ('<a i="j"><b><c a="b"></c></b><c></c><d><e>z</e><e>x</e></d></a>' | from xml)
    assert equal ($sample_xml | xupdate [* d e *] {|x| $x | update content 'nushell'}) ('<a><b><c a="b"></c></b><c></c><d><e>nushell</e><e>nushell</e></d></a>' | from xml)
    assert equal ($sample_xml | xupdate [* * * {|e| $e.attributes != {}}] {|x| $x | update content ['xml']}) {tag: a, attributes: {}, content: [[tag, attributes, content]; [b, {}, [[tag, attributes, content]; [c, {a: b}, [xml]]]], [c, {}, []], [d, {}, [[tag, attributes, content]; [e, {}, [[tag, attributes, content]; [null, null, z]]], [e, {}, [[tag, attributes, content]; [null, null, x]]]]]]}
}

@test
def xml_xinsert [] {
    let sample_xml = $in.sample_xml

    assert equal ($sample_xml | xinsert [a] {tag: b attributes:{} content: []}) ('<a><b><c a="b"></c></b><c></c><d><e>z</e><e>x</e></d><b></b></a>' | from xml)
    assert equal ($sample_xml | xinsert [a d *] {tag: null attributes: null content: 'n'} | to xml) '<a><b><c a="b"></c></b><c></c><d><e>zn</e><e>xn</e></d></a>'
    assert equal ($sample_xml | xinsert [a *] {tag: null attributes: null content: 'n'}) ('<a><b><c a="b"></c>n</b><c>n</c><d><e>z</e><e>x</e>n</d></a>' | from xml)
}
