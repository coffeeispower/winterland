value:
let
  dependencyAttributes = [
    "buildInputs"
    "nativeBuildInputs"
    "propagatedBuildInputs"
    "propagatedNativeBuildInputs"
    "depsBuildBuild"
    "depsBuildHost"
    "depsBuildTarget"
    "depsHostHost"
    "depsHostTarget"
    "depsTargetTarget"
    "checkInputs"
    "nativeCheckInputs"
    "installCheckInputs"
    "nativeInstallCheckInputs"
  ];
  concat = builtins.concatLists;
  walk = value:
    if builtins.isAttrs value && (value.type or null) == "derivation" then
      let
        meta = value.meta or {};
        winterland = meta.winterland or {};
        dependencies = dependencyAttributes
          |> map (name:
            if builtins.hasAttr name value
            then walk (builtins.getAttr name value)
            else []
          )
          |> concat;
      in [{
        drvPath = value.drvPath;
        source = meta.position or null;
        displayName = winterland.displayName or null;
          website = meta.homepage or null;
          repository = meta.repository or null;
          version = value.version or null;
      }] ++ dependencies
    else if builtins.isList value then
      value |> map walk |> concat
    else
      [];
in
  walk value
  |> map (entry: {
    name = builtins.unsafeDiscardStringContext entry.drvPath;
    value = {
      source = entry.source;
      displayName = entry.displayName;
          website = entry.website;
          repository = entry.repository;
          version = entry.version;
    };
  })
  |> builtins.listToAttrs
