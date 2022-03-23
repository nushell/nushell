@0xb299d30dc02d72bc;
# Schema representing all the structs that are used to communicate with
# the plugins.
# This schema, together with the command capnp proto is used to generate
# the rust file that defines the serialization/deserialization objects
# required to communicate with the plugins created for nushell
#
# If you modify the schema remember to compile it to generate the corresponding
# rust file and place that file into the main nu-plugin folder.
# After compiling, you may need to run cargo fmt on the file so it passes the CI

struct Err(T) {
	union {
		err @0 :Text;
		ok @1 :T;
	}
}

struct Map(Key, Value) {
  struct Entry {
    key @0 :Key;
    value @1 :Value;
  }
  entries @0 :List(Entry);
}

# Main plugin structures
struct Span {
	start @0 :UInt64;
	end @1 :UInt64;
}

# Resulting value from plugin
struct Value {
	span @0 :Span;

	union {
		void @1 :Void;
		bool @2 :Bool;
		int @3 :Int64;
		float @4 :Float64;
		string @5 :Text;
		list @6 :List(Value);
		record @7 :Record;
		filesize @8 :Int64;
		duration @9 :Int64;
		block @10 :Block;
		binary @11 :List(Int64);
		cellpath @12 :CellPath;
		range @13 :Range;
	}
}

struct HashMap {
	varid @0 :UInt64;
	value @1 :Value;
}

struct Block {
	blockid @0 :UInt64;
	captures @1 :List(HashMap);
}

struct Record {
	cols @0 :List(Text);
	vals @1 :List(Value);
}

struct CellPath {
	members @0 :List(PathMember);
}

struct PathMember {
	union {
		string @0: Value;
		int @1: Value;
	}
}

enum RangeInclusion {
	inclusive @0;
	rightexclusive @1;
}

struct Range {
	from @0 :Value;
	incr @1 :Value;
	to @2 :Value;
	inclusion @3 :RangeInclusion;
}

# Structs required to define the plugin signature
struct Signature {
    name @0 :Text;
    usage @1 :Text;
    extraUsage @2 :Text;
    requiredPositional @3 :List(Argument);
    optionalPositional @4 :List(Argument);
	# Optional value. Check for existence when deserializing
    rest @5 :Argument;
    named @6 :List(Flag);
    isFilter @7 :Bool;
	category @8 :Category;
}

enum Category {
	default @0;
	conversions @1;
	core @2;
	date @3;
	env @4;
	experimental @5;
	filesystem @6;
	filters @7;
	formats @8;
	math @9;
	network @10;
	random @11;
	platform @12;
	shells @13;
	strings @14;
	system @15;
	viewers @16;
	hash @17;
	generators @18;
}

struct Flag {
    long @0 :Text;
	# Optional value. Check for existence when deserializing (has_short)
    short @1 :Text;
    arg @2 :Shape;
    required @3 :Bool;
    desc @4 :Text;
}

struct Argument {
    name @0 :Text;
    desc @1 :Text;
    shape @2 :Shape;
}

# If we require more complex signatures for the plugins this could be
# changed to a union
enum Shape {
	none @0;
	any @1;
	string @2;
	number @3;
	int @4;
	boolean @5;
	cellPath @6;
	fullCellPath @7;
	range @8;
	filepath @9;
	globPattern @10;
	importPattern @11;
	binary @12;
	table @13;
	filesize @14;
	duration @15;
	dateTime @16;
	operator @17;
	rowCondition @18;
	mathExpression @19;
	variable @20;
	signature @21;
	expression @22;
	record @23;
	varWithOptType @24;
}

struct EvaluatedCall {
	head @0: Span;
	positional @1 :List(Value);
	# The value in the map can be optional
	# Check for existence when deserializing
	named @2 :Map(Text, Value);
}

struct CallInfo {
	name @0 :Text;
	call @1 :EvaluatedCall;
	input @2 :Value;
}

# Main communication structs with the plugin
struct PluginCall {
	union {
		signature @0 :Void;
		callInfo @1 :CallInfo;
	}
}

struct PluginResponse {
	union {
		error @0 :LabeledError;
		signature @1 :List(Signature);
		value @2 :Value;
	}
}

struct LabeledError {
	label @0 :Text;
	msg @1 :Text;
	# Optional Value. When decoding check if it exists (has_span)
	span @2 :Span;
}
