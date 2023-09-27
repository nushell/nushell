use std assert


# This is the custom command 1 for url_join:

#[test]
def url_join_outputs_a_url_representing_the_contents_of_this_record_1 [] {
  let result = ({
        "scheme": "http",
        "username": "",
        "password": "",
        "host": "www.pixiv.net",
        "port": "",
        "path": "/member_illust.php",
        "query": "mode=medium&illust_id=99260204",
        "fragment": "",
        "params":
        {
            "mode": "medium",
            "illust_id": "99260204"
        }
    } | url join)
  assert ($result == http://www.pixiv.net/member_illust.php?mode=medium&illust_id=99260204)
}

# This is the custom command 2 for url_join:

#[test]
def url_join_outputs_a_url_representing_the_contents_of_this_record_2 [] {
  let result = ({
        "scheme": "http",
        "username": "user",
        "password": "pwd",
        "host": "www.pixiv.net",
        "port": "1234",
        "query": "test=a",
        "fragment": ""
    } | url join)
  assert ($result == http://user:pwd@www.pixiv.net:1234?test=a)
}

# This is the custom command 3 for url_join:

#[test]
def url_join_outputs_a_url_representing_the_contents_of_this_record_3 [] {
  let result = ({
        "scheme": "http",
        "host": "www.pixiv.net",
        "port": "1234",
        "path": "user",
        "fragment": "frag"
    } | url join)
  assert ($result == http://www.pixiv.net:1234/user#frag)
}


