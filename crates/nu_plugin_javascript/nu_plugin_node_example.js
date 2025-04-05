#!/usr/bin/env node

/**
 * Node.js Nushell Plugin Example
 * Communicates with Nushell via JSON-encoded messages over stdin/stdout
 * 
 * Register command: `plugin add <path-to-js-file>`
 * Usage: node_example 2 "3"
 */

// Configuration constants
const NUSHELL_VERSION = '0.103.1';
const PLUGIN_VERSION = '0.1.1';

// Core protocol functions

/**
 * Writes a structured response to stdout
 * @param {number} id - Call identifier
 * @param {object} response - Response payload
 */
function writeResponse(id, response) {
	const message = JSON.stringify({ CallResponse: [id, response] });
	process.stdout.write(`${message}\n`);
}

/**
 * Writes an error response
 * @param {number} id - Call identifier
 * @param {string} text - Error description
 * @param {object|null} span - Error location metadata
 */
function writeError(id, text, span = null) {
	const error = span ? {
		Error: {
			msg: 'Plugin execution error',
			labels: [{ text, span }]
		}
	} : {
		Error: {
			msg: 'Plugin configuration error',
			help: text
		}
	};
	writeResponse(id, error);
}

// Plugin capability definitions

/**
 * Generates plugin signature metadata
 * @returns {object} Structured plugin capabilities
 */
function getPluginSignature() {
	return {
		Signature: [{
			sig: {
				name: 'node_example',
				description: 'Demonstration plugin for Node.js',
				extra_description: '',
				required_positional: [
					{
						name: 'a',
						desc: 'Required integer parameter',
						shape: 'Int'
					},
					{
						name: 'b',
						desc: 'Required string parameter',
						shape: 'String'
					}
				],
				optional_positional: [{
					name: 'opt',
					desc: 'Optional numeric parameter',
					shape: 'Int'
				}],
				rest_positional: {
					name: 'rest',
					desc: 'Variable-length string parameters',
					shape: 'String'
				},
				named: [
					{
						long: 'help',
						short: 'h',
						arg: null,
						required: false,
						desc: 'Display help information'
					},
					{
						long: 'flag',
						short: 'f',
						arg: null,
						required: false,
						desc: 'Example boolean flag'
					},
					{
						long: 'named',
						short: 'n',
						arg: 'String',
						required: false,
						desc: 'Example named parameter'
					}
				],
				input_output_types: [['Any', 'Any']],
				allow_variants_without_examples: true,
				search_terms: ['nodejs', 'example'],
				is_filter: false,
				creates_scope: false,
				allows_unknown_args: false,
				category: 'Experimental'
			},
			examples: []
		}]
	};
}

/**
 * Processes execution calls from Nushell
 * @param {number} id - Call identifier
 * @param {object} callData - Execution context metadata
 */
function processExecutionCall(id, callData) {
	const span = callData.call.head;

	// Generate sample tabular data
	const tableData = Array.from({ length: 10 }, (_, index) => ({
		Record: {
			val: {
				one: { Int: { val: index * 1, span } },
				two: { Int: { val: index * 2, span } },
				three: { Int: { val: index * 3, span } }
			},
			span
		}
	}));

	writeResponse(id, {
		PipelineData: {
			Value: [{
				List: {
					vals: tableData,
					span
				}
			}, null]
		}
	});
}

// Protocol handling

/**
 * Handles different types of input messages
 * @param {object} input - Parsed JSON message from Nushell
 */
function handleInputMessage(input) {
	if (input.Hello) {
		handleHelloMessage(input.Hello);
	} else if (input === 'Goodbye') {
		process.exit(0);
	} else if (input.Call) {
		handleCallMessage(...input.Call);
	} else if (input.Signal) {
		handleSignal(input.Signal);
	}
}

function handleHelloMessage({ version }) {
	if (version !== NUSHELL_VERSION) {
		process.stderr.write(`Version mismatch: Expected ${NUSHELL_VERSION}, got ${version}\n`);
		process.exit(1);
	}
}

function handleCallMessage(id, call) {
	try {
		if (call === 'Metadata') {
			writeResponse(id, { Metadata: { version: PLUGIN_VERSION } });
		} else if (call === 'Signature') {
			writeResponse(id, getPluginSignature());
		} else if (call.Run) {
			processExecutionCall(id, call.Run);
		} else {
			writeError(id, `Unsupported operation: ${JSON.stringify(call)}`);
		}
	} catch (error) {
		writeError(id, `Processing error: ${error.message}`);
	}
}

function handleSignal(signal) {
	if (signal !== 'Reset') {
		process.stderr.write(`Unhandled signal: ${signal}\n`);
	}
}

// Stream processing setup

/**
 * Initializes plugin communication protocol
 */
function initializePlugin() {
	// Set up JSON encoding
	process.stdout.write('\x04json\n');

	// Send handshake message
	process.stdout.write(JSON.stringify({
		Hello: {
			protocol: 'nu-plugin',
			version: NUSHELL_VERSION,
			features: []
		}
	}) + '\n');

	// Configure input processing
	let buffer = '';
	process.stdin.setEncoding('utf8')
		.on('data', chunk => {
			buffer += chunk;
			const messages = buffer.split('\n');
			buffer = messages.pop() || ''; // Preserve incomplete line

			for (const message of messages) {
				if (message.trim()) {
					try {
						handleInputMessage(JSON.parse(message));
					} catch (error) {
						process.stderr.write(`Parse error: ${error.message}\n`);
						process.stderr.write(`Received: ${message}\n`);
					}
				}
			}
		})
		.on('error', error => {
			process.stderr.write(`Input error: ${error.message}\n`);
			process.exit(1);
		});
}

// Main execution
if (process.argv.includes('--stdio')) {
	initializePlugin();
} else {
	console.log('This plugin is intended to be run from within Nushell');
	process.exit(2);
}
