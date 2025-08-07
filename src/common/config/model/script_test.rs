#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::config::model::script::Script;
    use serde_yaml::Value;

    #[test]
    fn test_script_without_shebang() {
        let script = Script("echo hello".to_string());
        let cmd = script.to_cmd().unwrap();
        
        assert_eq!(cmd.command, "sh");
        assert_eq!(cmd.args.len(), 2);
        assert_eq!(cmd.args[0], Value::String("-c".to_string()));
        assert_eq!(cmd.args[1], Value::String("echo hello".to_string()));
    }

    #[test]
    fn test_script_with_bash_env_shebang() {
        let script = Script("#!/usr/bin/env bash\necho hello".to_string());
        let cmd = script.to_cmd().unwrap();
        
        assert_eq!(cmd.command, "bash");
        assert_eq!(cmd.args.len(), 2);
        assert_eq!(cmd.args[0], Value::String("-c".to_string()));
        assert_eq!(cmd.args[1], Value::String("#!/usr/bin/env bash\necho hello".to_string()));
    }

    #[test]
    fn test_script_with_python_env_shebang() {
        let script = Script("#!/usr/bin/env python3\nprint('hello')".to_string());
        let cmd = script.to_cmd().unwrap();
        
        assert_eq!(cmd.command, "python3");
        assert_eq!(cmd.args.len(), 2);
        assert_eq!(cmd.args[0], Value::String("-c".to_string()));
        assert_eq!(cmd.args[1], Value::String("#!/usr/bin/env python3\nprint('hello')".to_string()));
    }

    #[test]
    fn test_script_with_absolute_path_shebang() {
        let script = Script("#!/bin/bash\necho hello".to_string());
        let cmd = script.to_cmd().unwrap();
        
        assert_eq!(cmd.command, "bash");
        assert_eq!(cmd.args.len(), 2);
        assert_eq!(cmd.args[0], Value::String("-c".to_string()));
        assert_eq!(cmd.args[1], Value::String("#!/bin/bash\necho hello".to_string()));
    }

    #[test]
    fn test_script_with_ruby_shebang() {
        let script = Script("#!/usr/bin/env ruby\nputs 'hello'".to_string());
        let cmd = script.to_cmd().unwrap();
        
        assert_eq!(cmd.command, "ruby");
        assert_eq!(cmd.args.len(), 2);
        assert_eq!(cmd.args[0], Value::String("-e".to_string()));
        assert_eq!(cmd.args[1], Value::String("#!/usr/bin/env ruby\nputs 'hello'".to_string()));
    }

    #[test]
    fn test_script_with_perl_shebang() {
        let script = Script("#!/usr/bin/env perl\nprint 'hello'".to_string());
        let cmd = script.to_cmd().unwrap();
        
        assert_eq!(cmd.command, "perl");
        assert_eq!(cmd.args.len(), 2);
        assert_eq!(cmd.args[0], Value::String("-e".to_string()));
        assert_eq!(cmd.args[1], Value::String("#!/usr/bin/env perl\nprint 'hello'".to_string()));
    }

    #[test]
    fn test_script_with_node_shebang() {
        let script = Script("#!/usr/bin/env node\nconsole.log('hello')".to_string());
        let cmd = script.to_cmd().unwrap();
        
        assert_eq!(cmd.command, "node");
        assert_eq!(cmd.args.len(), 2);
        assert_eq!(cmd.args[0], Value::String("-e".to_string()));
        assert_eq!(cmd.args[1], Value::String("#!/usr/bin/env node\nconsole.log('hello')".to_string()));
    }

    #[test]
    fn test_script_with_php_shebang() {
        let script = Script("#!/usr/bin/env php\necho 'hello';".to_string());
        let cmd = script.to_cmd().unwrap();
        
        assert_eq!(cmd.command, "php");
        assert_eq!(cmd.args.len(), 2);
        assert_eq!(cmd.args[0], Value::String("-r".to_string()));
        assert_eq!(cmd.args[1], Value::String("#!/usr/bin/env php\necho 'hello';".to_string()));
    }

    #[test]
    fn test_script_with_unknown_shebang() {
        let script = Script("#!/usr/bin/env unknownlang\nsome code".to_string());
        let cmd = script.to_cmd().unwrap();
        
        assert_eq!(cmd.command, "unknownlang");
        assert_eq!(cmd.args.len(), 2);
        assert_eq!(cmd.args[0], Value::String("-c".to_string()));
        assert_eq!(cmd.args[1], Value::String("#!/usr/bin/env unknownlang\nsome code".to_string()));
    }

    #[test]
    fn test_script_with_multiline_content() {
        let script_content = "#!/usr/bin/env bash\necho \"line 1\"\necho \"line 2\"\necho \"line 3\"";
        let script = Script(script_content.to_string());
        let cmd = script.to_cmd().unwrap();
        
        assert_eq!(cmd.command, "bash");
        assert_eq!(cmd.args.len(), 2);
        assert_eq!(cmd.args[0], Value::String("-c".to_string()));
        assert_eq!(cmd.args[1], Value::String(script_content.to_string()));
    }

    #[test]
    fn test_script_parse_shebang_method() {
        // Test the internal parse_shebang method
        let test_cases = vec![
            ("echo hello", ("sh", "echo hello")),
            ("#!/usr/bin/env bash\necho hello", ("bash", "#!/usr/bin/env bash\necho hello")),
            ("#!/bin/bash\necho hello", ("bash", "#!/bin/bash\necho hello")),
            ("#!/usr/bin/env python3\nprint('hello')", ("python3", "#!/usr/bin/env python3\nprint('hello')")),
            ("#!/usr/bin/env ruby\nputs 'hello'", ("ruby", "#!/usr/bin/env ruby\nputs 'hello'")),
            ("#!/usr/bin/env perl\nprint 'hello'", ("perl", "#!/usr/bin/env perl\nprint 'hello'")),
            ("#!/usr/bin/env node\nconsole.log('hello')", ("node", "#!/usr/bin/env node\nconsole.log('hello')")),
            ("#!/usr/bin/env php\necho 'hello';", ("php", "#!/usr/bin/env php\necho 'hello';")),
        ];

        for (input, expected) in test_cases {
            let script = Script(input.to_string());
            let (interpreter, content) = script.parse_shebang();
            assert_eq!((interpreter.as_str(), content.as_str()), expected);
        }
    }
}

