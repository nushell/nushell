@0xb299d30dc02d72bc;

struct Value {
	span @0: Span;

	struct Span {
		start @0 :UInt64;
		end @1 :UInt64;
	}

	union {
		void @1 :Void;
		bool @2 :Bool;
		int @3 :Int64;
		float @4 :Float64;
		string @5 :Text;
		list @6 :List(Value);
	}
}
