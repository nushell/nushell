@0xb299d30dc02d72bc;

# Generic structs used as helpers for the encoding
struct Option(Value) {
	union {
		none @0 :Void;
		some @1 :Value;
	}
}

struct Map(Key, Value) {
  struct Entry {
    key @0 :Key;
    value @1 :Value;
  }
  entries @0 :List(Entry);
}

struct Span {
	start @0 :UInt64;
	end @1 :UInt64;
}

struct Value {
	span @0: Span;

	union {
		void @1 :Void;
		bool @2 :Bool;
		int @3 :Int64;
		float @4 :Float64;
		string @5 :Text;
		list @6 :List(Value);
	}
}

struct Expression {
	union {
		garbage @0 :Void;
		bool @1 :Bool;
		int @2 :Int64;
		float @3 :Float64;
		string @4 :Text;
		list @5 :List(Expression);
	}
}

struct Call {
	head @0: Span;
	positional @1 :List(Expression);
	named @2 :Map(Text, Option(Expression));
}

struct CallInfo {
	call @0: Call;
	input @1: Value;
}
