use std/testing *
use std/assert

use std-rfc/xml [ xaccess ]

@before-each
def before-each [] {
    let sample_xml_doc = r#'
        <a>
            <b>
                <c a="b"></c>
            </b>
            <c></c>
            <d>
                <e>z</e>
                <e>x</e>
            </d>
        </a>
    '#

    {
        sample_xml: ($sample_xml_doc | from xml)
    }
}

@test
def "xaccess is backwards compatible" [] {
    let sample_xml = $in.sample_xml

    (
        assert equal
        ($sample_xml | xaccess [a])
        [$sample_xml]
    )
    (
        assert equal
        ($sample_xml | xaccess [*])
        [$sample_xml]
    )
    (
        assert equal
        ($sample_xml | xaccess [*, d, e])
        [
            [tag, attributes, content];
            [e, {}, [[tag, attributes, content]; [null, null, z]]]
            [e, {}, [[tag, attributes, content]; [null, null, x]]]
        ]
    )
    (
        assert equal
        ($sample_xml | xaccess [*, d, e, 1])
        [
            [tag, attributes, content];
            [e, {}, [[tag, attributes, content]; [null, null, x]]]
        ]
    )
    (
        assert equal
        ($sample_xml | xaccess [*, *, *, {|e| $e.attributes != {} }])
        [
            [tag, attributes, content];
            [c, {a: b}, []]
        ]
    )
}

@test
def "xaccess cell-path arguments work" [] {
    let sample_xml = $in.sample_xml

    (
        assert equal
        ($sample_xml | xaccess a)
        [$sample_xml]
    )
    (
        assert equal
        ($sample_xml | xaccess *)
        [$sample_xml]
    )
    (
        assert equal
        ($sample_xml | xaccess *.d.e)
        [
            [tag, attributes, content];
            [e, {}, [[tag, attributes, content]; [null, null, z]]]
            [e, {}, [[tag, attributes, content]; [null, null, x]]]
        ]
    )
    (
        assert equal
        ($sample_xml | xaccess *.d.e.1)
        [
            [tag, attributes, content];
            [e, {}, [[tag, attributes, content]; [null, null, x]]]
        ]
    )
    (
        assert equal
        ($sample_xml | xaccess *.*.* {|e| $e.attributes != {} })
        [
            [tag, attributes, content];
            [c, {a: b}, []]
        ]
    )
}

@test
def "xaccess descendant selector works" [] {
    let sample_xml = $in.sample_xml

    (
        assert equal
        ($sample_xml | xaccess **.e)
        [
            [tag, attributes, content];
            [e, {}, [[tag, attributes, content]; [null, null, z]]]
            [e, {}, [[tag, attributes, content]; [null, null, x]]]
        ]
    )
    (
        assert equal
        ($sample_xml | xaccess ** {|e| $e.attributes | is-not-empty })
        [
            [tag, attributes, content];
            [c, {a: b}, []]
        ]
    )
}
