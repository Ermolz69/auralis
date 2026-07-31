import assert from 'assert';
import { checkFileContent } from './check-runtime-println.mjs';

async function runTests() {
    console.log('Running tests for check-runtime-println.mjs...');

    // Test 1: File with dbg! should fail
    {
        const content = `fn main() {
            let x = 5;
            dbg!(x);
        }`;
        const hasError = await checkFileContent('test1.rs', content);
        assert.strictEqual(hasError, true, 'Expected dbg! to trigger an error');
    }

    // Test 2: File with println! should fail
    {
        const content = `fn main() {
            println!("Hello world");
        }`;
        const hasError = await checkFileContent('test2.rs', content);
        assert.strictEqual(hasError, true, 'Expected println! to trigger an error');
    }

    // Test 3: File with tracing::info! should pass
    {
        const content = `fn main() {
            tracing::info!("Hello world");
        }`;
        const hasError = await checkFileContent('test3.rs', content);
        assert.strictEqual(hasError, false, 'Expected tracing::info! to NOT trigger an error');
    }

    // Test 4: File with tracing::error! should pass
    {
        const content = `fn main() {
            tracing::error!("An error occurred");
        }`;
        const hasError = await checkFileContent('test4.rs', content);
        assert.strictEqual(hasError, false, 'Expected tracing::error! to NOT trigger an error');
    }

    // Test 5: File with std::io::stderr() in non-diagnostic file should fail
    {
        const content = `fn main() {
            let mut s = std::io::stderr();
        }`;
        const hasError = await checkFileContent('test5.rs', content);
        assert.strictEqual(hasError, true, 'Expected std::io::stderr to trigger an error in non-diagnostic file');
    }

    // Test 6: File with stderr() in non-diagnostic file should fail
    {
        const content = `fn main() {
            let mut s = stderr();
        }`;
        const hasError = await checkFileContent('test6.rs', content);
        assert.strictEqual(hasError, true, 'Expected stderr() to trigger an error in non-diagnostic file');
    }

    // Test 7: File with std::io::stderr() in diagnostic.rs should pass
    {
        const content = `fn main() {
            let mut s = std::io::stderr();
        }`;
        const hasError = await checkFileContent('src-tauri/src/observability/diagnostic.rs', content);
        assert.strictEqual(hasError, false, 'Expected std::io::stderr to pass in diagnostic.rs');
    }

    // Test 8: File with stderr() in diagnostic.rs should pass
    {
        const content = `fn main() {
            let mut s = stderr();
        }`;
        const hasError = await checkFileContent('src-tauri\\src\\observability\\diagnostic.rs', content);
        assert.strictEqual(hasError, false, 'Expected stderr() to pass in diagnostic.rs with windows path');
    }

    console.log('All tests passed!');
}

runTests().catch(err => {
    console.error('Tests failed:', err);
    process.exit(1);
});
