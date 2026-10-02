# A record whose key is not a plain string is typed `any` by nu, which passes as a block.
if true { $env.A:b }
