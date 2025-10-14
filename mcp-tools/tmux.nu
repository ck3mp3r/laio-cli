# Tmux management tool for nu-mcp - provides tmux session and pane control

# Default main command
def main [] {
  help main
}

# List available MCP tools
def "main list-tools" [] {
  [
    {
      name: "list_sessions"
      description: "List all tmux sessions with their windows and panes (returns tabular data)"
      input_schema: {
        type: "object"
        properties: {}
      }
    }
    {
      name: "send_command"
      description: "Send a command to a specific tmux pane"
      input_schema: {
        type: "object"
        properties: {
          session: {
            type: "string"
            description: "Session name or ID"
          }
          window: {
            type: "string"
            description: "Window name or ID (optional, defaults to current window)"
          }
          pane: {
            type: "string"
            description: "Pane ID (optional, defaults to current pane)"
          }
          command: {
            type: "string"
            description: "Command to send to the pane"
          }
        }
        required: ["session" "command"]
      }
    }
    {
      name: "capture_pane"
      description: "Capture and return the content of a specific tmux pane"
      input_schema: {
        type: "object"
        properties: {
          session: {
            type: "string"
            description: "Session name or ID"
          }
          window: {
            type: "string"
            description: "Window name or ID (optional, defaults to current window)"
          }
          pane: {
            type: "string"
            description: "Pane ID (optional, defaults to current pane)"
          }
          lines: {
            type: "integer"
            description: "Number of lines to capture (optional, defaults to all visible)"
          }
        }
        required: ["session"]
      }
    }
    {
      name: "get_session_info"
      description: "Get detailed information about a specific tmux session (returns tabular data)"
      input_schema: {
        type: "object"
        properties: {
          session: {
            type: "string"
            description: "Session name or ID"
          }
        }
        required: ["session"]
      }
    }
    {
      name: "get_pane_process"
      description: "Get information about the running process in a specific tmux pane (returns tabular data)"
      input_schema: {
        type: "object"
        properties: {
          session: {
            type: "string"
            description: "Session name or ID"
          }
          window: {
            type: "string"
            description: "Window name or ID (optional, defaults to current window)"
          }
          pane: {
            type: "string"
            description: "Pane ID (optional, defaults to current pane)"
          }
        }
        required: ["session"]
      }
    }
    {
      name: "find_pane_by_name"
      description: "Find a pane by its name across all windows in a session (returns tabular data)"
      input_schema: {
        type: "object"
        properties: {
          session: {
            type: "string"
            description: "Session name or ID"
          }
          pane_name: {
            type: "string"
            description: "Name of the pane to find"
          }
        }
        required: ["session" "pane_name"]
      }
    }
    {
      name: "find_pane_by_context"
      description: "Find a pane by context like directory path, command, or description. Useful for finding 'docs pane', 'build pane', etc. (returns tabular data)"
      input_schema: {
        type: "object"
        properties: {
          session: {
            type: "string"
            description: "Session name or ID"
          }
          context: {
            type: "string"
            description: "Context to search for: directory name (e.g. 'docs'), command (e.g. 'zola'), or description"
          }
        }
        required: ["session" "context"]
      }
    }
    {
      name: "list_panes"
      description: "List all panes in a session as a clear table showing window, pane, name, process, directory, and status (returns tabular data)"
      input_schema: {
        type: "object"
        properties: {
          session: {
            type: "string"
            description: "Session name or ID"
          }
        }
        required: ["session"]
      }
    }
    {
      name: "execute_and_capture"
      description: "Send a command to a tmux pane and immediately capture the output - useful for commands that need immediate feedback"
      input_schema: {
        type: "object"
        properties: {
          session: {
            type: "string"
            description: "Session name or ID"
          }
          window: {
            type: "string"
            description: "Window name or ID (optional, defaults to current window)"
          }
          pane: {
            type: "string"
            description: "Pane ID (optional, defaults to current pane)"
          }
          command: {
            type: "string"
            description: "Command to send to the pane"
          }
          wait_seconds: {
            type: "number"
            description: "Seconds to wait before capturing output (optional, defaults to 1)"
          }
          lines: {
            type: "integer"
            description: "Number of lines to capture (optional, defaults to all visible)"
          }
        }
        required: ["session" "command"]
      }
    }
  ] | to json
}

# Call a specific tool with arguments
def "main call-tool" [
  tool_name: string # Name of the tool to call
  args: any = {} # JSON arguments for the tool
] {
  let parsed_args = if ($args | describe) == "string" {
    $args | from json
  } else {
    $args
  }

  match $tool_name {
    "list_sessions" => {
      list_sessions
    }
    "send_command" => {
      let session = $parsed_args | get session
      let command = $parsed_args | get command
      let window = if "window" in $parsed_args { $parsed_args | get window } else { null }
      let pane = if "pane" in $parsed_args { $parsed_args | get pane } else { null }
      send_command $session $command $window $pane
    }
    "capture_pane" => {
      let session = $parsed_args | get session
      let window = if "window" in $parsed_args { $parsed_args | get window } else { null }
      let pane = if "pane" in $parsed_args { $parsed_args | get pane } else { null }
      let lines = if "lines" in $parsed_args { $parsed_args | get lines } else { null }
      capture_pane $session $window $pane $lines
    }
    "get_session_info" => {
      let session = $parsed_args | get session
      get_session_info $session
    }
    "get_pane_process" => {
      let session = $parsed_args | get session
      let window = if "window" in $parsed_args { $parsed_args | get window } else { null }
      let pane = if "pane" in $parsed_args { $parsed_args | get pane } else { null }
      get_pane_process $session $window $pane
    }
    "find_pane_by_name" => {
      let session = $parsed_args | get session
      let pane_name = $parsed_args | get pane_name
      find_pane_by_name $session $pane_name
    }
    "find_pane_by_context" => {
      let session = $parsed_args | get session
      let context = $parsed_args | get context
      find_pane_by_context $session $context
    }
    "list_panes" => {
      let session = $parsed_args | get session
      list_panes $session
    }
    "execute_and_capture" => {
      let session = $parsed_args | get session
      let command = $parsed_args | get command
      let window = if "window" in $parsed_args { $parsed_args | get window } else { null }
      let pane = if "pane" in $parsed_args { $parsed_args | get pane } else { null }
      let wait_seconds = if "wait_seconds" in $parsed_args { $parsed_args | get wait_seconds } else { 1 }
      let lines = if "lines" in $parsed_args { $parsed_args | get lines } else { null }
      execute_and_capture $session $command $window $pane $wait_seconds $lines
    }
    _ => {
      error make {msg: $"Unknown tool: ($tool_name)"}
    }
  }
}

# Check if tmux is available
def check_tmux [] {
  try {
    tmux -V | str trim
    true
  } catch {
    false
  }
}

# Helper function to execute tmux commands with logging (following k8s pattern)
def exec_tmux_command [cmd_args: list<string>] {
  let full_cmd = (["tmux"] | append $cmd_args)
  print $"Executing: ($full_cmd | str join ' ')"
  run-external ...$full_cmd
}

# List all tmux sessions with their windows and panes
def list_sessions [] {
  if not (check_tmux) {
    return "Error: tmux is not installed or not available in PATH"
  }

  try {
    # Get sessions
    let cmd_args = ["list-sessions" "-F" "#{session_name}|#{session_created}|#{session_attached}|#{session_windows}"]
    let sessions = exec_tmux_command $cmd_args | lines

    if ($sessions | length) == 0 {
      return "No tmux sessions found"
    }

    mut all_items = []

    for session_line in $sessions {
      let parts = $session_line | split row "|"
      let session_name = $parts | get 0
      let created = $parts | get 1
      let attached = $parts | get 2
      let window_count = $parts | get 3

      let status = if $attached == "1" { "attached" } else { "detached" }

      # Get windows for this session
      let cmd_args = ["list-windows" "-t" $session_name "-F" "#{window_index}|#{window_name}|#{window_panes}"]
      let windows = exec_tmux_command $cmd_args | lines

      for window_line in $windows {
        let window_parts = $window_line | split row "|"
        let window_index = $window_parts | get 0
        let window_name = $window_parts | get 1
        let pane_count = $window_parts | get 2

        # Get panes for this window
        let cmd_args = ["list-panes" "-t" $"($session_name):($window_index)" "-F" "#{pane_index}|#{pane_current_command}|#{pane_active}|#{pane_title}"]
        let panes = exec_tmux_command $cmd_args | lines

        for pane_line in $panes {
          let pane_parts = $pane_line | split row "|"
          let pane_index = $pane_parts | get 0
          let current_command = $pane_parts | get 1
          let is_active = $pane_parts | get 2
          let pane_title = $pane_parts | get 3

          let pane_status = if $is_active == "1" { "active" } else { "inactive" }
          let title = if $pane_title != "" { $pane_title } else { "" }

          $all_items = (
            $all_items | append {
              session: $session_name
              session_status: $status
              window: $window_index
              window_name: $window_name
              pane: $pane_index
              pane_title: $title
              command: $current_command
              pane_status: $pane_status
            }
          )
        }
      }
    }

    $all_items | table
  } catch {
    "Error: Failed to list tmux sessions. Make sure tmux is running."
  }
}

# Send a command to a specific tmux pane
def send_command [session: string command: string window?: string pane?: string] {
  if not (check_tmux) {
    return "Error: tmux is not installed or not available in PATH"
  }

  try {
    # Resolve the target (supports pane names)
    let target = resolve_pane_target $session $window $pane
    if $target == null {
      return $"Error: Could not find pane '($pane)' in session '($session)'"
    }

    # Send the command
    let cmd_args = ["send-keys" "-t" $target $command "Enter"]
    exec_tmux_command $cmd_args
    $"Command sent to ($target): ($command)"
  } catch {
    $"Error: Failed to send command to tmux session/pane. Check that the session '($session)' exists."
  }
}

# Capture content from a specific tmux pane
def capture_pane [session: string window?: string pane?: string lines?: int] {
  if not (check_tmux) {
    return "Error: tmux is not installed or not available in PATH"
  }

  try {
    # Resolve the target (supports pane names)
    let target = resolve_pane_target $session $window $pane
    if $target == null {
      return $"Error: Could not find pane '($pane)' in session '($session)'"
    }

    # Build capture command
    mut cmd_args = ["capture-pane" "-t" $target "-p"]
    if $lines != null {
      $cmd_args = ($cmd_args | append ["-S" $"-($lines)"])
    }

    # Capture the pane content
    let content = exec_tmux_command $cmd_args | str trim

    $"Pane content from ($target):\n---\n($content)\n---"
  } catch {
    $"Error: Failed to capture pane content. Check that the session/pane '($session)' exists."
  }
}

# Get detailed information about a specific tmux session
def get_session_info [session: string] {
  if not (check_tmux) {
    return "Error: tmux is not installed or not available in PATH"
  }

  try {
    # Get session info
    let cmd_args = ["display-message" "-t" $session "-p" "#{session_name}|#{session_created}|#{session_attached}|#{session_windows}|#{session_group}|#{session_id}"]
    let session_info = exec_tmux_command $cmd_args | str trim
    let parts = $session_info | split row "|"

    let session_name = $parts | get 0
    let created_timestamp = $parts | get 1
    let attached = $parts | get 2
    let window_count = $parts | get 3
    let session_group = $parts | get 4
    let session_id = $parts | get 5

    let status = if $attached == "1" { "attached" } else { "detached" }
    let created_date = $created_timestamp | into int | into datetime

    mut output = [
      $"Session Information for: ($session_name)"
      $"Session ID: ($session_id)"
      $"Status: ($status)"
      $"Created: ($created_date)"
      $"Windows: ($window_count)"
    ]

    if $session_group != "" {
      $output = ($output | append $"Group: ($session_group)")
    }

    $output = ($output | append "")

    # Get all panes across all windows as a table
    let cmd_args = ["list-windows" "-t" $session "-F" "#{window_index}|#{window_name}|#{window_active}"]
    let windows = exec_tmux_command $cmd_args | lines

    mut all_panes = []

    for window_line in $windows {
      let window_parts = $window_line | split row "|"
      let window_index = $window_parts | get 0
      let window_name = $window_parts | get 1
      let window_is_active = $window_parts | get 2

      # Get detailed pane information for this window
      let cmd_args = ["list-panes" "-t" $"($session):($window_index)" "-F" "#{pane_index}|#{pane_title}|#{pane_current_command}|#{pane_active}|#{pane_current_path}|#{pane_pid}"]
      let panes = exec_tmux_command $cmd_args | lines

      for pane_line in $panes {
        let pane_parts = $pane_line | split row "|"
        let pane_index = $pane_parts | get 0
        let pane_title = $pane_parts | get 1
        let current_command = $pane_parts | get 2
        let pane_is_active = $pane_parts | get 3
        let current_path = $pane_parts | get 4
        let pane_pid = $pane_parts | get 5

        # Determine custom name vs auto-generated title
        let looks_auto_generated = (
          ($pane_title | str contains "> ") or
          ($pane_title | str contains "✳") or
          ($pane_title | str contains "/") or
          ($pane_title == $current_path) or
          ($pane_title == "")
        )

        let custom_name = if $looks_auto_generated or ($pane_title | str length) > 20 {
          ""
        } else {
          $pane_title
        }

        let status = if $window_is_active == "1" and $pane_is_active == "1" {
          "active"
        } else if $pane_is_active == "1" {
          "current"
        } else {
          "inactive"
        }

        let pane_record = {
          window: $window_index
          window_name: $window_name
          pane: $pane_index
          name: $custom_name
          process: $current_command
          directory: ($current_path | path basename)
          full_path: $current_path
          pid: $pane_pid
          status: $status
          target: $"($session):($window_index).($pane_index)"
        }

        $all_panes = ($all_panes | append $pane_record)
      }
    }

    # Create expanded nested table structure
    $output = ($output | append "Windows and Panes:")
    $output = ($output | append "")

    # Use group-by to create proper nested table with expansion
    let nested_table = $all_panes | select window window_name pane process directory status | group-by window window_name --to-table | update items {|row| $row.items | select pane process directory status }

    let table_output = $nested_table | table --expand
    $output = ($output | append $table_output)

    $output | str join (char newline)
  } catch {
    $"Error: Failed to get session info for '($session)'. Check that the session exists."
  }
}

# Helper function to resolve pane target (supports pane names and context)
def resolve_pane_target [session: string window?: string pane?: string] {
  # If pane looks like a name (not just numbers), try to find it by name or context
  if $pane != null and not ($pane =~ '^[0-9]+$') {
    # First try finding by explicit name
    let find_result = find_pane_by_name $session $pane
    if ($find_result | str starts-with "Found pane") {
      # Extract target from the result
      let target_line = $find_result | lines | where ($it | str starts-with "Target:") | first
      let target = $target_line | str replace "Target: " ""
      return $target
    }

    # If name search failed, try context search
    let context_result = find_pane_by_context $session $pane
    if ($context_result | str starts-with "Found pane") {
      # Extract target from the result
      let target_line = $context_result | lines | where ($it | str starts-with "Target:") | first
      let target = $target_line | str replace "Target: " ""
      return $target
    }

    return null
  }

  # Build target using window/pane IDs
  mut target = $session
  if $window != null {
    $target = $"($target):($window)"
  }
  if $pane != null {
    if $window == null {
      $target = $"($target):"
    }
    $target = $"($target).($pane)"
  }
  return $target
}

# Get information about the running process in a specific tmux pane
def get_pane_process [session: string window?: string pane?: string] {
  if not (check_tmux) {
    return "Error: tmux is not installed or not available in PATH"
  }

  try {
    # Resolve the target (supports pane names)
    let target = resolve_pane_target $session $window $pane
    if $target == null {
      return $"Error: Could not find pane '($pane)' in session '($session)'"
    }

    # Get pane information including PID and command
    let cmd_args = ["display-message" "-t" $target "-p" "#{pane_index}|#{pane_current_command}|#{pane_pid}|#{pane_current_path}|#{pane_width}x#{pane_height}|#{pane_active}"]
    let pane_info = exec_tmux_command $cmd_args | str trim
    let parts = $pane_info | split row "|"

    let pane_index = $parts | get 0
    let current_command = $parts | get 1
    let pane_pid = $parts | get 2
    let current_path = $parts | get 3
    let pane_size = $parts | get 4
    let is_active = $parts | get 5

    let active_status = if $is_active == "1" { "active" } else { "inactive" }

    # Try to get more detailed process information
    let process_info = try {
      run-external "ps" "-p" $pane_pid "-o" "pid,ppid,command" | lines | skip 1 | first
    } catch {
      $"PID ($pane_pid): ($current_command)"
    }

    [
      {
        target: $target
        pane_index: $pane_index
        status: $active_status
        size: $pane_size
        current_path: $current_path
        current_command: $current_command
        process_id: $pane_pid
        process_details: $process_info
      }
    ] | table
  } catch {
    $"Error: Failed to get pane process info for '($session)'. Check that the session/pane exists."
  }
}

# Find a pane by its name/title across all windows in a session
def find_pane_by_name [session: string pane_name: string] {
  if not (check_tmux) {
    return "Error: tmux is not installed or not available in PATH"
  }

  try {
    # Get all windows in the session
    let cmd_args = ["list-windows" "-t" $session "-F" "#{window_index}"]
    let windows = exec_tmux_command $cmd_args | lines

    mut found_panes = []

    for window_index in $windows {
      # Get panes in this window with their titles
      let cmd_args = ["list-panes" "-t" $"($session):($window_index)" "-F" "#{pane_index}|#{pane_title}|#{pane_current_command}|#{pane_active}|#{pane_current_path}"]
      let panes = exec_tmux_command $cmd_args | lines

      for pane_line in $panes {
        let parts = $pane_line | split row "|"
        let pane_index = $parts | get 0
        let pane_title = $parts | get 1
        let current_command = $parts | get 2
        let is_active = $parts | get 3
        let current_path = $parts | get 4

        # Check if this pane matches the name (case-insensitive)
        if ($pane_title | str downcase) == ($pane_name | str downcase) {
          let active_status = if $is_active == "1" { "active" } else { "inactive" }
          let pane_info = {
            session: $session
            window: $window_index
            pane: $pane_index
            title: $pane_title
            command: $current_command
            status: $active_status
            path: $current_path
            target: $"($session):($window_index).($pane_index)"
          }
          $found_panes = ($found_panes | append $pane_info)
        }
      }
    }

    if ($found_panes | length) == 0 {
      $"No pane named '($pane_name)' found in session '($session)'"
    } else {
      $found_panes | select session window pane title command status path target | table
    }
  } catch {
    $"Error: Failed to search for pane '($pane_name)' in session '($session)'. Check that the session exists."
  }
}

# Find a pane by context (directory, command, description)
def find_pane_by_context [session: string context: string] {
  if not (check_tmux) {
    return "Error: tmux is not installed or not available in PATH"
  }

  try {
    # Get all windows in the session
    let cmd_args = ["list-windows" "-t" $session "-F" "#{window_index}"]
    let windows = exec_tmux_command $cmd_args | lines

    mut found_panes = []
    let search_context = $context | str downcase

    for window_index in $windows {
      # Get panes in this window with detailed info
      let cmd_args = ["list-panes" "-t" $"($session):($window_index)" "-F" "#{pane_index}|#{pane_title}|#{pane_current_command}|#{pane_active}|#{pane_current_path}"]
      let panes = exec_tmux_command $cmd_args | lines

      for pane_line in $panes {
        let parts = $pane_line | split row "|"
        let pane_index = $parts | get 0
        let pane_title = $parts | get 1
        let current_command = $parts | get 2
        let is_active = $parts | get 3
        let current_path = $parts | get 4

        # Check if context matches any of: title, command, path segment, or directory name
        let title_lower = $pane_title | str downcase
        let command_lower = $current_command | str downcase
        let path_lower = $current_path | str downcase
        let dir_name = $current_path | path basename | str downcase

        let matches = (
          ($title_lower | str contains $search_context) or
          ($command_lower | str contains $search_context) or
          ($path_lower | str contains $search_context) or
          ($dir_name == $search_context)
        )

        if $matches {
          let active_status = if $is_active == "1" { "active" } else { "inactive" }
          let pane_info = {
            session: $session
            window: $window_index
            pane: $pane_index
            title: $pane_title
            command: $current_command
            status: $active_status
            path: $current_path
            target: $"($session):($window_index).($pane_index)"
          }
          $found_panes = ($found_panes | append $pane_info)
        }
      }
    }

    if ($found_panes | length) == 0 {
      $"No pane matching context '($context)' found in session '($session)'"
    } else {
      $found_panes | select session window pane title command status path target | table
    }
  } catch {
    $"Error: Failed to search for context '($context)' in session '($session)'. Check that the session exists."
  }
}

# Execute a command and capture the output with intelligent back-off polling
def execute_and_capture [session: string command: string window?: string pane?: string wait_seconds: number = 1 lines?: int] {
  # Capture initial state before sending command
  let initial_result = capture_pane $session $window $pane $lines
  if ($initial_result | str starts-with "Error:") {
    return $initial_result
  }
  let initial_content = $initial_result | lines | skip 2 | drop 1 | str join (char newline) | str trim

  # Send the command
  let send_result = send_command $session $command $window $pane
  if ($send_result | str starts-with "Error:") {
    return $send_result
  }

  # Poll for output with exponential back-off
  mut attempt = 0
  mut delay_ms = 100 # Start with 100ms
  let max_attempts = 10
  let max_wait_ms = ($wait_seconds * 1000) | into int
  mut total_waited_ms = 0

  loop {
    # Wait before capturing
    sleep ($delay_ms | into duration --unit ms)
    $total_waited_ms = $total_waited_ms + $delay_ms

    # Capture current content
    let capture_result = capture_pane $session $window $pane $lines
    if ($capture_result | str starts-with "Error:") {
      return $capture_result
    }
    let current_content = $capture_result | lines | skip 2 | drop 1 | str join (char newline) | str trim

    # Check if content has meaningfully changed
    let content_changed = ($current_content != $initial_content)
    let has_new_lines = ($current_content | lines | length) > ($initial_content | lines | length)
    let content_grew = ($current_content | str length) > ($initial_content | str length) + 10

    # Stop if we have meaningful new content or hit limits
    if $content_changed and ($has_new_lines or $content_grew) {
      let waited_sec = ($total_waited_ms / 1000.0)
      return $"Command executed: ($command)\nPolled for ($waited_sec) seconds until output appeared\n---\n($current_content)\n---"
    }

    $attempt = $attempt + 1

    # Check if we should give up
    if $attempt >= $max_attempts or $total_waited_ms >= $max_wait_ms {
      let waited_sec = ($total_waited_ms / 1000.0)
      if $current_content == $initial_content {
        return $"Command executed: ($command)\nNo new output detected after ($waited_sec) seconds\n---\n($current_content)\n---"
      } else {
        return $"Command executed: ($command)\nTimeout reached after ($waited_sec) seconds\n---\n($current_content)\n---"
      }
    }

    # Exponential back-off: 100ms, 150ms, 225ms, 337ms, 505ms, 757ms, 1000ms...
    if $delay_ms < 1000 {
      $delay_ms = ([$delay_ms * 1.5 1000] | math min) | into int
    }
  }
}

# List all panes in a session as a table
def list_panes [session: string] {
  if not (check_tmux) {
    return "Error: tmux is not installed or not available in PATH"
  }

  try {
    # Get all windows
    let cmd_args = ["list-windows" "-t" $session "-F" "#{window_index}|#{window_name}|#{window_active}"]
    let windows = exec_tmux_command $cmd_args | lines

    mut all_panes = []

    for window_line in $windows {
      let window_parts = $window_line | split row "|"
      let window_index = $window_parts | get 0
      let window_name = $window_parts | get 1
      let window_is_active = $window_parts | get 2

      # Get detailed pane information for this window
      let cmd_args = ["list-panes" "-t" $"($session):($window_index)" "-F" "#{pane_index}|#{pane_title}|#{pane_current_command}|#{pane_active}|#{pane_current_path}|#{pane_pid}"]
      let panes = exec_tmux_command $cmd_args | lines

      for pane_line in $panes {
        let pane_parts = $pane_line | split row "|"
        let pane_index = $pane_parts | get 0
        let pane_title = $pane_parts | get 1
        let current_command = $pane_parts | get 2
        let pane_is_active = $pane_parts | get 3
        let current_path = $pane_parts | get 4
        let pane_pid = $pane_parts | get 5

        # Determine custom name vs auto-generated title
        let looks_auto_generated = (
          ($pane_title | str contains "> ") or
          ($pane_title | str contains "✳") or
          ($pane_title | str contains "/") or
          ($pane_title == $current_path) or
          ($pane_title == "")
        )

        let custom_name = if $looks_auto_generated or ($pane_title | str length) > 20 {
          ""
        } else {
          $pane_title
        }

        let status = if $window_is_active == "1" and $pane_is_active == "1" {
          "active"
        } else if $pane_is_active == "1" {
          "current"
        } else {
          "inactive"
        }

        let pane_record = {
          window: $window_index
          window_name: $window_name
          pane: $pane_index
          name: $custom_name
          process: $current_command
          directory: ($current_path | path basename)
          full_path: $current_path
          pid: $pane_pid
          status: $status
          target: $"($session):($window_index).($pane_index)"
        }

        $all_panes = ($all_panes | append $pane_record)
      }
    }

    # Create proper nested table structure with expansion
    $all_panes | select window window_name pane process directory status | group-by window window_name --to-table | update items {|row| $row.items | select pane process directory status } | table --expand
  } catch {
    $"Error: Failed to list panes for session '($session)'. Check that the session exists."
  }
}
