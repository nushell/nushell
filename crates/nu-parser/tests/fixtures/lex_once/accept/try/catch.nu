try { error make {msg: "x"} } catch { |e| print $e.msg }
try { 1 } catch { 2 }
try { 1 } catch { |e| }
try { 1 } catch {|| 2 }
