def test1 [ --a: any = 32 ] {}
def test2 [ --a: number = 32 ] {}
def test3 [ --a: number = 32.0 ] {}
def test4 [ --a: list<any> = [ 1 2 3 ] ] {}
def test5 [ --a: record<a: int b: string> = { a: 32 b: 'qwe' c: 'wqe' } ] {}
def test6 [ --a: record<a: any b: any> = { a: 32 b: 'qwe'} ] {}
