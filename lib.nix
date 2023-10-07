let
  extractParts = sys:
    let
      parts = builtins.match "([a-z0-9_]+)-([a-z]+)" sys;
    in
    {
      arch = builtins.elemAt parts 0;
      platform = builtins.elemAt parts 1;
    };
in
{
  inherit extractParts;
}
