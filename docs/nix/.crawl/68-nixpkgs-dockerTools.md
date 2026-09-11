---
type: Crawl Source
title: "nixpkgs — dockerTools"
description: "Docker image building tools in nixpkgs."
resource: https://nixos.org/manual/nixpkgs/stable/#ssec-pkgs-dockerTools
tags: [nix, nixpkgs-manual, dockerTools, buildImage, buildLayeredImage, streamLayeredImage]
timestamp: 2026-07-24T00:00:00Z
---

# Crawl: nixpkgs-dockerTools

- seed_url: https://nixos.org/manual/nixpkgs/stable/
- canonical_url: https://nixos.org/manual/nixpkgs/stable/
- family: nixpkgs Manual
- fetch: 200
- version: nixpkgs 26.05
- feeds_docs: nix-docker-images.md

## Content

## pkgs.dockerTools 

[buildImage](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildImage>)
[buildLayeredImage](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildLayeredImage>)
[streamLayeredImage](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-streamLayeredImage>)
[pullImage](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-pullImage>)
[exportImage](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-exportImage>)
[Environment Helpers](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-helpers>)
[buildNixShellImage](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildNixShellImage>)
[streamNixShellImage](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-streamNixShellImage>)

`pkgs.dockerTools` is a set of functions for creating and manipulating Docker images according to the [Docker Image Specification v1.3.1](<https://github.com/moby/docker-image-spec/blob/v1.3.1/spec.md>). Docker itself is not used to perform any of the operations done by these functions.

### buildImage 

This function builds a Docker-compatible repository tarball containing a single image. As such, the result is suitable for being loaded in Docker with `docker image load` (see [Example 351](</manual/nixpkgs/stable/#ex-dockerTools-buildImage> "Example 351. Building a Docker image") for how to do this).

This function will create a single layer for all files (and dependencies) that are specified in its argument. Only new dependencies that are not already in the existing layers will be copied. If you prefer to create multiple layers for the files and dependencies you want to add to the image, see [the section called “buildLayeredImage”](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildLayeredImage> "buildLayeredImage") or [the section called “streamLayeredImage”](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-streamLayeredImage> "streamLayeredImage") instead.

This function allows a script to be run during the layer generation process, allowing custom behaviour to affect the final results of the image (see the documentation of the `runAsRoot` and `extraCommands` attributes).

The resulting repository tarball will list a single image as specified by the `name` and `tag` attributes. By default, that image will use a static creation date (see documentation for the `created` attribute). This allows `buildImage` to produce reproducible images.

### Tip

When running an image built with `buildImage`, you might encounter certain errors depending on what you included in the image, especially if you did not start with any base image.

If you encounter errors similar to `getProtocolByName: does not exist (no such protocol name: tcp)`, you may need to add the contents of `pkgs.iana-etc` in the `copyToRoot` attribute. Similarly, if you encounter errors similar to `Error_Protocol ("certificate has unknown CA",True,UnknownCa)`, you may need to add the contents of `pkgs.cacert` in the `copyToRoot` attribute.

#### Inputs 

`buildImage` expects an argument with the following attributes:

`name` (String)
    

The name of the generated image.

`tag` (String or Null; _optional_)
    

Tag of the generated image. If `null`, the hash of the nix derivation will be used as the tag.

_Default value:_ `null`.

`fromImage` (Path or Null; _optional_)
    

The repository tarball of an image to be used as the base for the generated image. It must be a valid Docker image, such as one exported by `docker image save`, or another image built with the `dockerTools` utility functions. This can be seen as an equivalent of `FROM fromImage` in a `Dockerfile`. A value of `null` can be seen as an equivalent of `FROM scratch`.

If specified, the layer created by `buildImage` will be appended to the layers defined in the base image, resulting in an image with at least two layers (one or more layers from the base image and the layer created by `buildImage`). Otherwise, the resulting image will contain the single layer created by `buildImage`.

### Note

Only **Env** configuration is inherited from the base image.

_Default value:_ `null`.

`fromImageName` (String or Null; _optional_)
    

Used to specify the image within the repository tarball in case it contains multiple images. A value of `null` means that `buildImage` will use the first image available in the repository.

### Note

This must be used with `fromImageTag`. Using only `fromImageName` without `fromImageTag` will make `buildImage` use the first image available in the repository.

_Default value:_ `null`.

`fromImageTag` (String or Null; _optional_)
    

Used to specify the image within the repository tarball in case it contains multiple images. A value of `null` means that `buildImage` will use the first image available in the repository.

### Note

This must be used with `fromImageName`. Using only `fromImageTag` without `fromImageName` will make `buildImage` use the first image available in the repository.

_Default value:_ `null`.

`copyToRoot` (Path, List of Paths, or Null; _optional_)
    

Files to add to the generated image. Anything that coerces to a path (e.g. a derivation) can also be used. This can be seen as an equivalent of `ADD contents/ /` in a `Dockerfile`.

_Default value:_ `null`.

`keepContentsDirlinks` (Boolean; _optional_)
    

When adding files to the generated image (as specified by `copyToRoot`), this attribute controls whether to preserve symlinks to directories. If `false`, the symlinks will be transformed into directories. This behaves the same as `rsync -k` when `keepContentsDirlinks` is `false`, and the same as `rsync -K` when `keepContentsDirlinks` is `true`.

_Default value:_ `false`.

`runAsRoot` (String or Null; _optional_)
    

A bash script that will run as root inside a VM that contains the existing layers of the base image and the new generated layer (including the files from `copyToRoot`). The script will be run with a working directory of `/`. This can be seen as an equivalent of `RUN ...` in a `Dockerfile`. A value of `null` means that this step in the image generation process will be skipped.

See [Example 352](</manual/nixpkgs/stable/#ex-dockerTools-buildImage-runAsRoot> "Example 352. Building a Docker image with runAsRoot") for how to work with this attribute.

### Caution

Using this attribute requires the `kvm` device to be available, see [`system-features`](<https://nixos.org/manual/nix/stable/command-ref/conf-file.html#conf-system-features>). If the `kvm` device isn’t available, you should consider using [`buildLayeredImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildLayeredImage> "buildLayeredImage") or [`streamLayeredImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-streamLayeredImage> "streamLayeredImage") instead. Those functions allow scripts to be run as root without access to the `kvm` device.

### Note

At the time the script in `runAsRoot` is run, the files specified directly in `copyToRoot` will be present in the VM, but their dependencies might not be there yet. Copying their dependencies into the generated image is a step that happens after `runAsRoot` finishes running.

_Default value:_ `null`.

`extraCommands` (String; _optional_)
    

A bash script that will run before the layer created by `buildImage` is finalised. The script will be run on some (opaque) working directory which will become `/` once the layer is created. This is similar to `runAsRoot`, but the script specified in `extraCommands` is **not** run as root, and does not involve creating a VM. It is simply run as part of building the derivation that outputs the layer created by `buildImage`.

See [Example 353](</manual/nixpkgs/stable/#ex-dockerTools-buildImage-extraCommands> "Example 353. Building a Docker image with extraCommands") for how to work with this attribute, and subtle differences compared to `runAsRoot`.

_Default value:_ `""`.

`config` (Attribute Set or Null; _optional_)
    

Used to specify the configuration of the containers that will be started off the generated image. Must be an attribute set, with each attribute as listed in the [Docker Image Specification v1.3.1](<https://github.com/moby/docker-image-spec/blob/v1.3.1/spec.md#image-json-field-descriptions>).

_Default value:_ `null`.

`architecture` (String; _optional_)
    

Used to specify the image architecture. This is useful for multi-architecture builds that don’t need cross compiling. If specified, its value should follow the [OCI Image Configuration Specification](<https://github.com/opencontainers/image-spec/blob/v1.1.1/config.md#properties>), which should still be compatible with Docker. According to the linked specification, all possible values for `$GOARCH` in [the Go docs](<https://go.dev/doc/install/source#environment>) should be valid, but will commonly be one of `386`, `amd64`, `arm`, or `arm64`.

_Default value:_ the same value from `pkgs.go.GOARCH`.

`diskSize` (Number; _optional_)
    

Controls the disk size in MiB (1024x1024 bytes) of the VM used to run the script specified in `runAsRoot`. This attribute is ignored if `runAsRoot` is `null`.

_Default value:_ 1024.

`buildVMMemorySize` (Number; _optional_)
    

Controls the amount of memory in MiB (1024x1024 bytes) provisioned for the VM used to run the script specified in `runAsRoot`. This attribute is ignored if `runAsRoot` is `null`.

_Default value:_ 512.

`created` (String; _optional_)
    

Specifies the time of creation of the generated image. This should be either a date and time formatted according to [ISO-8601](<https://en.wikipedia.org/wiki/ISO_8601>) or `"now"`, in which case `buildImage` will use the current date.

See [Example 354](</manual/nixpkgs/stable/#ex-dockerTools-buildImage-creatednow> "Example 354. Building a Docker image with a creation date set to the current time") for how to use `"now"`.

### Caution

Using `"now"` means that the generated image will not be reproducible anymore (because the date will always change whenever it’s built).

_Default value:_ `"1970-01-01T00:00:01Z"`.

`uid` (Number; _optional_)
    

The uid of the user that will own the files packed in the new layer built by `buildImage`.

_Default value:_ 0.

`gid` (Number; _optional_)
    

The gid of the group that will own the files packed in the new layer built by `buildImage`.

_Default value:_ 0.

`compressor` (String; _optional_)
    

Selects the algorithm used to compress the image.

_Default value:_ `"gz"`.  
_Possible values:_ `"none"`, `"gz"`, `"zstd"`.

`includeNixDB` (Boolean; _optional_)
    

Populate the nix database in the image with the dependencies of `copyToRoot`. The main purpose is to be able to use nix commands in the container.

### Caution

Be careful since this doesn’t work well in combination with `fromImage`. In particular, in a multi-layered image, only the Nix paths from the lower image will be in the database.

This also neglects to register the store paths that are pulled into the image as a dependency of one of the other values, but aren’t a dependency of `copyToRoot`.

_Default value:_ `false`.

`meta` (Attribute Set)
    

The `meta` attribute of the resulting derivation, as in `stdenv.mkDerivation`. Accepts `description`, `maintainers` and any other `meta` attributes.

`contents` **DEPRECATED**
    

This attribute is deprecated, and users are encouraged to use `copyToRoot` instead.

#### Passthru outputs 

`buildImage` defines a few [`passthru`](</manual/nixpkgs/stable/#chap-passthru> "Passthru-attributes") attributes:

`buildArgs` (Attribute Set)
    

The argument passed to `buildImage` itself. This allows you to inspect all attributes specified in the argument, as described above.

`layer` (Attribute Set)
    

The derivation with the layer created by `buildImage`. This allows easier inspection of the contents added by `buildImage` in the generated image.

`imageTag` (String)
    

The tag of the generated image. This is useful if no tag was specified in the attributes of the argument to `buildImage`, because an automatic tag will be used instead. `imageTag` allows you to retrieve the value of the tag used in this case.

#### Examples 

**Example 351. Building a Docker image**

The following package builds a Docker image that runs the `redis-server` executable from the `redis` package. The Docker image will have name `redis` and tag `latest`.
    
    
    {
      dockerTools,
      buildEnv,
      redis,
    }:
    dockerTools.buildImage {
      name = "redis";
      tag = "latest";
    
      copyToRoot = buildEnv {
        name = "image-root";
        paths = [ redis ];
        pathsToLink = [ "/bin" ];
      };
    
      runAsRoot = ''
        mkdir -p /data
      '';
    
      config = {
        Cmd = [ "/bin/redis-server" ];
        WorkingDir = "/data";
        Volumes = {
          "/data" = { };
        };
      };
    }
    

The result of building this package is a `.tar.gz` file that can be loaded into Docker:
    
    
    $ nix-build
    (some output removed for clarity)
    building '/nix/store/yw0adm4wpsw1w6j4fb5hy25b3arr9s1v-docker-image-redis.tar.gz.drv'...
    Adding layer...
    tar: Removing leading `/' from member names
    Adding meta...
    Cooking the image...
    Finished.
    /nix/store/p4dsg62inh9d2ksy3c7bv58xa851dasr-docker-image-redis.tar.gz
    
    $ docker image load -i /nix/store/p4dsg62inh9d2ksy3c7bv58xa851dasr-docker-image-redis.tar.gz
    (some output removed for clarity)
    Loaded image: redis:latest
    

  

**Example 352. Building a Docker image with`runAsRoot`**

The following package builds a Docker image with the `hello` executable from the `hello` package. It uses `runAsRoot` to create a directory and a file inside the image.

This works the same as [Example 353](</manual/nixpkgs/stable/#ex-dockerTools-buildImage-extraCommands> "Example 353. Building a Docker image with extraCommands"), but uses `runAsRoot` instead of `extraCommands`.
    
    
    {
      dockerTools,
      buildEnv,
      hello,
    }:
    dockerTools.buildImage {
      name = "hello";
      tag = "latest";
    
      copyToRoot = buildEnv {
        name = "image-root";
        paths = [ hello ];
        pathsToLink = [ "/bin" ];
      };
    
      runAsRoot = ''
        mkdir -p /data
        echo "some content" > my-file
      '';
    
      config = {
        Cmd = [ "/bin/hello" ];
        WorkingDir = "/data";
      };
    }
    

  

**Example 353. Building a Docker image with`extraCommands`**

The following package builds a Docker image with the `hello` executable from the `hello` package. It uses `extraCommands` to create a directory and a file inside the image.

This works the same as [Example 352](</manual/nixpkgs/stable/#ex-dockerTools-buildImage-runAsRoot> "Example 352. Building a Docker image with runAsRoot"), but uses `extraCommands` instead of `runAsRoot`. Note that with `extraCommands`, we can’t directly reference `/` and must create files and directories as if we were already on `/`.
    
    
    {
      dockerTools,
      buildEnv,
      hello,
    }:
    dockerTools.buildImage {
      name = "hello";
      tag = "latest";
    
      copyToRoot = buildEnv {
        name = "image-root";
        paths = [ hello ];
        pathsToLink = [ "/bin" ];
      };
    
      extraCommands = ''
        mkdir -p data
        echo "some content" > my-file
      '';
    
      config = {
        Cmd = [ "/bin/hello" ];
        WorkingDir = "/data";
      };
    }
    

  

**Example 354. Building a Docker image with a creation date set to the current time**

Note that using a value of `"now"` in the `created` attribute will break reproducibility.
    
    
    {
      dockerTools,
      buildEnv,
      hello,
    }:
    dockerTools.buildImage {
      name = "hello";
      tag = "latest";
    
      created = "now";
    
      copyToRoot = buildEnv {
        name = "image-root";
        paths = [ hello ];
        pathsToLink = [ "/bin" ];
      };
    
      config.Cmd = [ "/bin/hello" ];
    }
    

After importing the generated repository tarball with Docker, its CLI will display a reasonable date and sort the images as expected:
    
    
    $ docker image ls
    REPOSITORY   TAG      IMAGE ID       CREATED              SIZE
    hello        latest   de2bf4786de6   About a minute ago   25.2MB
    

  

### buildLayeredImage 

`buildLayeredImage` uses [`streamLayeredImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-streamLayeredImage> "streamLayeredImage") underneath to build a compressed Docker-compatible repository tarball. Basically, `buildLayeredImage` runs the script created by `streamLayeredImage` to save the compressed image in the Nix store. `buildLayeredImage` supports the same options as `streamLayeredImage`, see [`streamLayeredImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-streamLayeredImage> "streamLayeredImage") for details.

### Note

Despite the similar name, [`buildImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildImage> "buildImage") works completely differently from `buildLayeredImage` and `streamLayeredImage`.

Even though some of the arguments may seem related, they cannot be interchanged.

You can load the result of this function in Docker with `docker image load`. See [Example 355](</manual/nixpkgs/stable/#ex-dockerTools-buildLayeredImage-hello> "Example 355. Building a layered Docker image") to see how to do that.

#### Examples 

**Example 355. Building a layered Docker image**

The following package builds a layered Docker image that runs the `hello` executable from the `hello` package. The Docker image will have name `hello` and tag `latest`.
    
    
    { dockerTools, hello }:
    dockerTools.buildLayeredImage {
      name = "hello";
      tag = "latest";
    
      contents = [ hello ];
    
      config.Cmd = [ "/bin/hello" ];
    }
    

The result of building this package is a `.tar.gz` file that can be loaded into Docker:
    
    
    $ nix-build
    (some output removed for clarity)
    building '/nix/store/bk8bnrbw10nq7p8pvcmdr0qf57y6scha-hello.tar.gz.drv'...
    No 'fromImage' provided
    Creating layer 1 from paths: ['/nix/store/i93s7xxblavsacpy82zdbn4kplsyq48l-libunistring-1.1']
    Creating layer 2 from paths: ['/nix/store/ji01n9vinnj22nbrb86nx8a1ssgpilx8-libidn2-2.3.4']
    Creating layer 3 from paths: ['/nix/store/ldrslljw4rg026nw06gyrdwl78k77vyq-xgcc-12.3.0-libgcc']
    Creating layer 4 from paths: ['/nix/store/9y8pmvk8gdwwznmkzxa6pwyah52xy3nk-glibc-2.38-27']
    Creating layer 5 from paths: ['/nix/store/zhl06z4lrfrkw5rp0hnjjfrgsclzvxpm-hello-2.12.1']
    Creating layer 6 with customisation...
    Adding manifests...
    Done.
    /nix/store/hxcz7snvw7f8rzhbh6mv8jq39d992905-hello.tar.gz
    
    $ docker image load -i /nix/store/hxcz7snvw7f8rzhbh6mv8jq39d992905-hello.tar.gz
    (some output removed for clarity)
    Loaded image: hello:latest
    

  

### streamLayeredImage 

`streamLayeredImage` builds a **script** which, when run, will stream to stdout a Docker-compatible repository tarball containing a single image, using multiple layers to improve sharing between images. This means that `streamLayeredImage` does not output an image into the Nix store, but only a script that builds the image, saving on IO and disk/cache space, particularly with large images.

You can load the result of this function in Docker with `docker image load`. See [Example 356](</manual/nixpkgs/stable/#ex-dockerTools-streamLayeredImage-hello> "Example 356. Streaming a layered Docker image") to see how to do that.

For this function, you specify a [store path](<https://nixos.org/manual/nix/stable/store/store-path>) or a list of store paths to be added to the image, and the functions will automatically include any dependencies of those paths in the image. The function will attempt to create one layer per object in the Nix store that needs to be added to the image. In case there are more objects to include than available layers, the function will put the most [“popular”](<https://github.com/NixOS/nixpkgs/tree/release-23.11/pkgs/build-support/references-by-popularity>) objects in their own layers, and group all remaining objects into a single layer.

An additional layer will be created with symlinks to the store paths you specified to be included in the image. These symlinks are built with [`symlinkJoin`](</manual/nixpkgs/stable/#trivial-builder-symlinkJoin> "symlinkJoin"), so they will be included in the root of the image. See [Example 357](</manual/nixpkgs/stable/#ex-dockerTools-streamLayeredImage-exploringlayers> "Example 357. Exploring the layers in an image built with streamLayeredImage") to understand how these symlinks are laid out in the generated image.

`streamLayeredImage` allows scripts to be run when creating the additional layer with symlinks, allowing custom behaviour to affect the final results of the image (see the documentation of the `extraCommands` and `fakeRootCommands` attributes).

The resulting repository tarball will list a single image as specified by the `name` and `tag` attributes. By default, that image will use a static creation date (see documentation for the `created` and `mtime` attributes). This allows the function to produce reproducible images.

#### Inputs 

`streamLayeredImage` expects one argument with the following attributes:

`name` (String)
    

The name of the generated image.

`tag` (String or Null; _optional_)
    

Tag of the generated image. If `null`, the hash of the nix derivation will be used as the tag.

_Default value:_ `null`.

`fromImage`(Path or Null; _optional_)
    

The repository tarball of an image to be used as the base for the generated image. It must be a valid Docker image, such as one exported by `docker image save`, or another image built with the `dockerTools` utility functions. This can be seen as an equivalent of `FROM fromImage` in a `Dockerfile`. A value of `null` can be seen as an equivalent of `FROM scratch`.

If specified, the created layers will be appended to the layers defined in the base image.

_Default value:_ `null`.

`contents` (Path or List of Paths; _optional_) 
    

Directories whose contents will be added to the generated image. Things that coerce to paths (e.g. a derivation) can also be used. This can be seen as an equivalent of `ADD contents/ /` in a `Dockerfile`.

All the contents specified by `contents` will be added as a final layer in the generated image. They will be added as links to the actual files (e.g., links to the store paths). The actual files will be added in previous layers.

_Default value:_ `[]`

`config` (Attribute Set or Null; _optional_) 
    

Used to specify the configuration of the containers that will be started off the generated image. Must be an attribute set, with each attribute as listed in the [Docker Image Specification v1.3.0](<https://github.com/moby/moby/blob/46f7ab808b9504d735d600e259ca0723f76fb164/image/spec/spec.md#image-json-field-descriptions>).

If any packages are used directly in `config`, they will be automatically included in the generated image. See [Example 358](</manual/nixpkgs/stable/#ex-dockerTools-streamLayeredImage-configclosure> "Example 358. Building a layered Docker image with packages directly in config") for an example.

_Default value:_ `null`.

`architecture` (String; _optional_)
    

Used to specify the image architecture. This is useful for multi-architecture builds that don’t need cross compiling. If specified, its value should follow the [OCI Image Configuration Specification](<https://github.com/opencontainers/image-spec/blob/main/config.md#properties>), which should still be compatible with Docker. According to the linked specification, all possible values for `$GOARCH` in [the Go docs](<https://go.dev/doc/install/source#environment>) should be valid, but will commonly be one of `386`, `amd64`, `arm`, or `arm64`.

_Default value:_ the same value from `pkgs.go.GOARCH`.

`created` (String; _optional_)
    

Specifies the time of creation of the generated image. This date will be used for the image metadata. This should be either a date and time formatted according to [ISO-8601](<https://en.wikipedia.org/wiki/ISO_8601>) or `"now"`, in which case the current date will be used.

### Caution

Using `"now"` means that the generated image will not be reproducible anymore (because the date will always change whenever it’s built).

_Default value:_ `"1970-01-01T00:00:01Z"`.

`mtime` (String; _optional_)
    

Specifies the time used for the modification timestamp of files within the layers of the generated image. This should be either a date and time formatted according to [ISO-8601](<https://en.wikipedia.org/wiki/ISO_8601>) or `"now"`, in which case the current date will be used.

### Caution

Using a non-constant date will cause built layers to have a different hash each time, preventing deduplication. Using `"now"` also means that the generated image will not be reproducible anymore (because the date will always change whenever it’s built).

_Default value:_ `"1970-01-01T00:00:01Z"`.

`uid` (Number; _optional_)  `gid` (Number; _optional_)  `uname` (String; _optional_)  `gname` (String; _optional_) 

: Credentials for Nix store ownership. Can be overridden to, e.g., `1000` / `1000` / `"user"` / `"user"` to enable building a container where Nix can be used as an unprivileged user in single-user mode.

_Default value:_ `0` / `0` / `"root"` / `"root"`

`maxLayers` (Number; _optional_) 
    

The maximum number of layers that will be used by the generated image. If a `fromImage` was specified, the number of layers used by `fromImage` will be subtracted from `maxLayers` to ensure that the image generated will have at most `maxLayers`.

### Caution

Depending on the tool/runtime where the image will be used, there might be a limit to the number of layers that an image can have. For Docker, see [this issue on GitHub](<https://github.com/docker/docs/issues/8230>).

_Default value:_ 100.

`extraCommands` (String; _optional_)
    

A bash script that will run in the context of the layer created with the contents specified by `contents`. At the moment this script runs, only the contents directly specified by `contents` will be available as links.

_Default value:_ `""`.

`fakeRootCommands` (String; _optional_)
    

A bash script that will run in the context of the layer created with the contents specified by `contents`. During the process to generate that layer, the script in `extraCommands` will be run first, if specified. After that, a fakeroot(1) environment will be entered. The script specified in `fakeRootCommands` runs inside the fakeroot environment, and the layer is then generated from the view of the files inside the fakeroot environment.

This is useful to change the owners of the files in the layer (by running `chown`, for example), or performing any other privileged operations related to file manipulation (by default, all files in the layer will be owned by root, and the build environment doesn’t have enough privileges to directly perform privileged operations on these files).

For more details, see the manpage for fakeroot(1).

### Caution

Due to how fakeroot works, static binaries cannot perform privileged file operations in `fakeRootCommands`, unless `enableFakechroot` is set to `true`.

_Default value:_ `""`.

`enableFakechroot` (Boolean; _optional_)
    

By default, the script specified in `fakeRootCommands` only runs inside a fakeroot environment. If `enableFakechroot` is `true`, a more complete chroot environment will be created using [`proot`](<https://proot-me.github.io/>) before running the script in `fakeRootCommands`. Files in the Nix store will be available. This allows scripts that perform installation in `/` to work as expected. This can be seen as an equivalent of `RUN ...` in a `Dockerfile`.

_Default value:_ `false`

`includeStorePaths` (Boolean; _optional_)
    

The files specified in `contents` are put into layers in the generated image. If `includeStorePaths` is `false`, the actual files will not be included in the generated image, and only links to them will be added instead. It is **not recommended** to set this to `false` unless you have other tooling to insert the store paths via other means (such as bind mounting the host store) when running containers with the generated image. If you don’t provide any extra tooling, the generated image won’t run properly.

See [Example 357](</manual/nixpkgs/stable/#ex-dockerTools-streamLayeredImage-exploringlayers> "Example 357. Exploring the layers in an image built with streamLayeredImage") to understand the impact of setting `includeStorePaths` to `false`.

_Default value:_ `true`

`includeNixDB` (Boolean; _optional_)
    

Populate the nix database in the image with the dependencies of `copyToRoot`. The main purpose is to be able to use nix commands in the container.

### Caution

Be careful since this doesn’t work well in combination with `fromImage`. In particular, in a multi-layered image, only the Nix paths from the lower image will be in the database.

This also neglects to register the store paths that are pulled into the image as a dependency of one of the other values, but aren’t a dependency of `copyToRoot`.

_Default value:_ `false`.

`meta` (Attribute Set)
    

The `meta` attribute of the resulting derivation, as in `stdenv.mkDerivation`. Accepts `description`, `maintainers` and any other `meta` attributes.

`passthru` (Attribute Set; _optional_)
    

Use this to pass any attributes as [`passthru`](</manual/nixpkgs/stable/#chap-passthru> "Passthru-attributes") for the resulting derivation.

_Default value:_ `{}`

#### Passthru outputs 

`streamLayeredImage` also defines its own [`passthru`](</manual/nixpkgs/stable/#chap-passthru> "Passthru-attributes") attributes:

`imageTag` (String)
    

The tag of the generated image. This is useful if no tag was specified in the attributes of the argument to the function, because an automatic tag will be used instead. `imageTag` allows you to retrieve the value of the tag used in this case.

#### Examples 

**Example 356. Streaming a layered Docker image**

The following package builds a **script** which, when run, will stream a layered Docker image that runs the `hello` executable from the `hello` package. The Docker image will have name `hello` and tag `latest`.
    
    
    { dockerTools, hello }:
    dockerTools.streamLayeredImage {
      name = "hello";
      tag = "latest";
    
      contents = [ hello ];
    
      config.Cmd = [ "/bin/hello" ];
    }
    

The result of building this package is a script. Running this script and piping it into `docker image load` gives you the same image that was built in [Example 355](</manual/nixpkgs/stable/#ex-dockerTools-buildLayeredImage-hello> "Example 355. Building a layered Docker image"). Note that in this case, the image is never added to the Nix store, but instead streamed directly into Docker.
    
    
    $ nix-build
    (output removed for clarity)
    /nix/store/wsz2xl8ckxnlb769irvq6jv1280dfvxd-stream-hello
    
    $ /nix/store/wsz2xl8ckxnlb769irvq6jv1280dfvxd-stream-hello | docker image load
    No 'fromImage' provided
    Creating layer 1 from paths: ['/nix/store/i93s7xxblavsacpy82zdbn4kplsyq48l-libunistring-1.1']
    Creating layer 2 from paths: ['/nix/store/ji01n9vinnj22nbrb86nx8a1ssgpilx8-libidn2-2.3.4']
    Creating layer 3 from paths: ['/nix/store/ldrslljw4rg026nw06gyrdwl78k77vyq-xgcc-12.3.0-libgcc']
    Creating layer 4 from paths: ['/nix/store/9y8pmvk8gdwwznmkzxa6pwyah52xy3nk-glibc-2.38-27']
    Creating layer 5 from paths: ['/nix/store/zhl06z4lrfrkw5rp0hnjjfrgsclzvxpm-hello-2.12.1']
    Creating layer 6 with customisation...
    Adding manifests...
    Done.
    (some output removed for clarity)
    Loaded image: hello:latest
    

  

**Example 357. Exploring the layers in an image built with`streamLayeredImage`**

Assume the following package, which builds a layered Docker image with the `hello` package.
    
    
    { dockerTools, hello }:
    dockerTools.streamLayeredImage {
      name = "hello";
      contents = [ hello ];
    }
    

The `hello` package depends on 4 other packages:
    
    
    $ nix-store --query -R $(nix-build -A hello)
    /nix/store/i93s7xxblavsacpy82zdbn4kplsyq48l-libunistring-1.1
    /nix/store/ji01n9vinnj22nbrb86nx8a1ssgpilx8-libidn2-2.3.4
    /nix/store/ldrslljw4rg026nw06gyrdwl78k77vyq-xgcc-12.3.0-libgcc
    /nix/store/9y8pmvk8gdwwznmkzxa6pwyah52xy3nk-glibc-2.38-27
    /nix/store/zhl06z4lrfrkw5rp0hnjjfrgsclzvxpm-hello-2.12.1
    

This means that all these packages will be included in the image generated by `streamLayeredImage`. It will put each package in its own layer, for a total of 5 layers with actual files in them. A final layer will be created only with symlinks for the `hello` package.

The image generated will have the following directory structure (some directories were collapsed for readability):
    
    
    ├── bin
    │   └── hello → /nix/store/zhl06z4lrfrkw5rp0hnjjfrgsclzvxpm-hello-2.12.1/bin/hello
    ├── nix
    │   └── store
    │       ├─⊕ 9y8pmvk8gdwwznmkzxa6pwyah52xy3nk-glibc-2.38-27
    │       ├─⊕ i93s7xxblavsacpy82zdbn4kplsyq48l-libunistring-1.1
    │       ├─⊕ ji01n9vinnj22nbrb86nx8a1ssgpilx8-libidn2-2.3.4
    │       ├─⊕ ldrslljw4rg026nw06gyrdwl78k77vyq-xgcc-12.3.0-libgcc
    │       └─⊕ zhl06z4lrfrkw5rp0hnjjfrgsclzvxpm-hello-2.12.1
    └── share
        ├── info
        │   └── hello.info → /nix/store/zhl06z4lrfrkw5rp0hnjjfrgsclzvxpm-hello-2.12.1/share/info/hello.info
        ├─⊕ locale
        └── man
            └── man1
                └── hello.1.gz → /nix/store/zhl06z4lrfrkw5rp0hnjjfrgsclzvxpm-hello-2.12.1/share/man/man1/hello.1.gz
    

Each of the packages in `/nix/store` comes from a layer in the image. The final layer adds the `/bin` and `/share` directories, but they only contain links to the actual files in `/nix/store`.

If our package sets `includeStorePaths` to `false`, we’ll end up with only the final layer with the links, but the actual files won’t exist in the image:
    
    
    { dockerTools, hello }:
    dockerTools.streamLayeredImage {
      name = "hello";
      contents = [ hello ];
      includeStorePaths = false;
    }
    

After building this package, the image will have the following directory structure:
    
    
    ├── bin
    │   └── hello → /nix/store/zhl06z4lrfrkw5rp0hnjjfrgsclzvxpm-hello-2.12.1/bin/hello
    └── share
        ├── info
        │   └── hello.info → /nix/store/zhl06z4lrfrkw5rp0hnjjfrgsclzvxpm-hello-2.12.1/share/info/hello.info
        ├─⊕ locale
        └── man
            └── man1
                └── hello.1.gz → /nix/store/zhl06z4lrfrkw5rp0hnjjfrgsclzvxpm-hello-2.12.1/share/man/man1/hello.1.gz
    

Note how the links point to paths in `/nix/store`, but they’re not included in the image itself. This is why you need extra tooling when using `includeStorePaths`: a container created from such image won’t find any of the files it needs to run otherwise.

  

**Example 358. Building a layered Docker image with packages directly in`config`**

The closure of `config` is automatically included in the generated image. The following package shows a more compact way to create the same output generated in [Example 356](</manual/nixpkgs/stable/#ex-dockerTools-streamLayeredImage-hello> "Example 356. Streaming a layered Docker image").
    
    
    {
      dockerTools,
      hello,
      lib,
    }:
    dockerTools.streamLayeredImage {
      name = "hello";
      tag = "latest";
      config.Cmd = [ "${lib.getExe hello}" ];
    }
    

  

### pullImage 

This function is similar to the `docker image pull` command, which means it can be used to pull a Docker image from a registry that implements the [Docker Registry HTTP API V2](<https://distribution.github.io/distribution/spec/api/>). By default, the `docker.io` registry is used.

The image will be downloaded as an uncompressed Docker-compatible repository tarball, which is suitable for use with other `dockerTools` functions such as [`buildImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildImage> "buildImage"), [`buildLayeredImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildLayeredImage> "buildLayeredImage"), and [`streamLayeredImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-streamLayeredImage> "streamLayeredImage").

This function requires two different types of hashes/digests to be specified:

  * One of them is used to identify a unique image within the registry (see the documentation for the `imageDigest` attribute).

  * The other is used by Nix to ensure the contents of the output haven’t changed (see the documentation for the `sha256` attribute).

Both hashes are required because they must uniquely identify some content in two completely different systems (the Docker registry and the Nix store), but their values will not be the same. See [Example 361](</manual/nixpkgs/stable/#ex-dockerTools-pullImage-nixprefetchdocker> "Example 361. Finding the digest and hash values to use for dockerTools.pullImage") for a tool that can help gather these values.

#### Inputs 

`pullImage` expects a single argument with the following attributes:

`imageName` (String)
    

Specifies the name of the image to be downloaded, as well as the registry endpoint. By default, the `docker.io` registry is used. To specify a different registry, prepend the endpoint to `imageName`, separated by a slash (`/`). See [Example 360](</manual/nixpkgs/stable/#ex-dockerTools-pullImage-differentregistry> "Example 360. Pulling the nixos/nix Docker image from a specific registry") for how to do that.

`imageDigest` (String)
    

Specifies the digest of the image to be downloaded.

### Tip

**Why can’t I specify a tag to pull from, and have to use a digest instead?**

Tags are often updated to point to different image contents. The most common example is the `latest` tag, which is usually updated whenever a newer image version is available.

An image tag isn’t enough to guarantee the contents of an image won’t change, but a digest guarantees this. Providing a digest helps ensure that you will still be able to build the same Nix code and get the same output even if newer versions of an image are released.

`sha256` (String)
    

The hash of the image after it is downloaded. Internally, this is passed to the [`outputHash`](<https://nixos.org/manual/nix/stable/language/advanced-attributes#adv-attr-outputHash>) attribute of the resulting derivation. This is needed to provide a guarantee to Nix that the contents of the image haven’t changed, because Nix doesn’t support the value in `imageDigest`.

`finalImageName` (String; _optional_)
    

Specifies the name that will be used for the image after it has been downloaded. This only applies after the image is downloaded, and is not used to identify the image to be downloaded in the registry. Use `imageName` for that instead.

_Default value:_ the same value specified in `imageName`.

`finalImageTag` (String; _optional_)
    

Specifies the tag that will be used for the image after it has been downloaded. This only applies after the image is downloaded, and is not used to identify the image to be downloaded in the registry.

_Default value:_ `"latest"`.

`os` (String; _optional_)
    

Specifies the operating system of the image to pull. If specified, its value should follow the [OCI Image Configuration Specification](<https://github.com/opencontainers/image-spec/blob/main/config.md#properties>), which should still be compatible with Docker. According to the linked specification, all possible values for `$GOOS` in [the Go docs](<https://go.dev/doc/install/source#environment>) should be valid, but will commonly be one of `darwin` or `linux`.

_Default value:_ `"linux"`.

`arch` (String; _optional_)
    

Specifies the architecture of the image to pull. If specified, its value should follow the [OCI Image Configuration Specification](<https://github.com/opencontainers/image-spec/blob/main/config.md#properties>), which should still be compatible with Docker. According to the linked specification, all possible values for `$GOARCH` in [the Go docs](<https://go.dev/doc/install/source#environment>) should be valid, but will commonly be one of `386`, `amd64`, `arm`, or `arm64`.

_Default value:_ the same value from `pkgs.go.GOARCH`.

`tlsVerify` (Boolean; _optional_)
    

Used to enable or disable HTTPS and TLS certificate verification when communicating with the chosen Docker registry. Setting this to `false` will make `pullImage` connect to the registry through HTTP.

_Default value:_ `true`.

`name` (String; _optional_)
    

The name used for the output in the Nix store path.

_Default value:_ a value derived from `finalImageName` and `finalImageTag`, with some symbols replaced. It is recommended to treat the default as an opaque value.

#### Examples 

**Example 359. Pulling the nixos/nix Docker image from the default registry**

This example pulls the [`nixos/nix` image](<https://hub.docker.com/r/nixos/nix>) and saves it in the Nix store.
    
    
    { dockerTools }:
    dockerTools.pullImage {
      imageName = "nixos/nix";
      imageDigest = "sha256:b8ea88f763f33dfda2317b55eeda3b1a4006692ee29e60ee54ccf6d07348c598";
      finalImageName = "nix";
      finalImageTag = "2.19.3";
      hash = "sha256-zRwlQs1FiKrvHPaf8vWOR/Tlp1C5eLn1d9pE4BZg3oA=";
    }
    

  

**Example 360. Pulling the nixos/nix Docker image from a specific registry**

This example pulls the [`coreos/etcd` image](<https://quay.io/repository/coreos/etcd>) from the `quay.io` registry.
    
    
    { dockerTools }:
    dockerTools.pullImage {
      imageName = "quay.io/coreos/etcd";
      imageDigest = "sha256:24a23053f29266fb2731ebea27f915bb0fb2ae1ea87d42d890fe4e44f2e27c5d";
      finalImageName = "etcd";
      finalImageTag = "v3.5.11";
      hash = "sha256-Myw+85f2/EVRyMB3axECdmQ5eh9p1q77FWYKy8YpRWU=";
    }
    

  

**Example 361. Finding the digest and hash values to use for`dockerTools.pullImage`**

Since [`dockerTools.pullImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-pullImage> "pullImage") requires two different hashes, one can run the `nix-prefetch-docker` tool to find out the values for the hashes. The tool outputs some text for an attribute set which you can pass directly to `pullImage`.
    
    
    $ nix run nixpkgs#nix-prefetch-docker -- --image-name nixos/nix --image-tag 2.19.3 --arch amd64 --os linux
    (some output removed for clarity)
    Writing manifest to image destination
    -> ImageName: nixos/nix
    -> ImageDigest: sha256:498fa2d7f2b5cb3891a4edf20f3a8f8496e70865099ba72540494cd3e2942634
    -> FinalImageName: nixos/nix
    -> FinalImageTag: latest
    -> ImagePath: /nix/store/4mxy9mn6978zkvlc670g5703nijsqc95-docker-image-nixos-nix-latest.tar
    -> ImageHash: 1q6cf2pdrasa34zz0jw7pbs6lvv52rq2aibgxccbwcagwkg2qj1q
    {
      imageName = "nixos/nix";
      imageDigest = "sha256:498fa2d7f2b5cb3891a4edf20f3a8f8496e70865099ba72540494cd3e2942634";
      hash = "sha256-OEgs3uRPMb4Y629FJXAWZW9q9LqHS/A/GUqr3K5wzOA=";
      finalImageName = "nixos/nix";
      finalImageTag = "latest";
    }
    

It is important to supply the `--arch` and `--os` arguments to `nix-prefetch-docker` to filter to a single image, in case there are multiple architectures and/or operating systems supported by the image name and tags specified. By default, `nix-prefetch-docker` will set `os` to `linux` and `arch` to `amd64`.

Run `nix-prefetch-docker --help` for a list of all supported arguments:
    
    
    $ nix run nixpkgs#nix-prefetch-docker -- --help
    (output removed for clarity)
    

  

### exportImage 

This function is similar to the `docker container export` command, which means it can be used to export an image’s filesystem as an uncompressed tarball archive. The difference is that `docker container export` is applied to containers, but `dockerTools.exportImage` applies to Docker images. The resulting archive will not contain any image metadata (such as command to run with `docker container run`), only the filesystem contents.

You can use this function to import an archive in Docker with `docker image import`. See [Example 363](</manual/nixpkgs/stable/#ex-dockerTools-exportImage-importingDocker> "Example 363. Importing an archive built with dockerTools.exportImage in Docker") to understand how to do that.

### Caution

`exportImage` works by unpacking the given image inside a VM. Because of this, using this function requires the `kvm` device to be available, see [`system-features`](<https://nixos.org/manual/nix/stable/command-ref/conf-file.html#conf-system-features>).

#### Inputs 

`exportImage` expects an argument with the following attributes:

`fromImage` (Attribute Set or String)
    

The repository tarball of the image whose filesystem will be exported. It must be a valid Docker image, such as one exported by `docker image save`, or another image built with the `dockerTools` utility functions.

If `name` is not specified, `fromImage` must be an Attribute Set corresponding to a derivation, i.e. it can’t be a path to a tarball. If `name` is specified, `fromImage` can be either an Attribute Set corresponding to a derivation or simply a path to a tarball.

See [Example 364](</manual/nixpkgs/stable/#ex-dockerTools-exportImage-naming> "Example 364. Exploring output naming with dockerTools.exportImage") and [Example 365](</manual/nixpkgs/stable/#ex-dockerTools-exportImage-fromImagePath> "Example 365. Using dockerTools.exportImage with a path as fromImage") to understand the connection between `fromImage`, `name`, and the name used for the output of `exportImage`.

`fromImageName` (String or Null; _optional_)
    

Used to specify the image within the repository tarball in case it contains multiple images. A value of `null` means that `exportImage` will use the first image available in the repository.

### Note

This must be used with `fromImageTag`. Using only `fromImageName` without `fromImageTag` will make `exportImage` use the first image available in the repository.

_Default value:_ `null`.

`fromImageTag` (String or Null; _optional_)
    

Used to specify the image within the repository tarball in case it contains multiple images. A value of `null` means that `exportImage` will use the first image available in the repository.

### Note

This must be used with `fromImageName`. Using only `fromImageTag` without `fromImageName` will make `exportImage` use the first image available in the repository.

_Default value:_ `null`.

`diskSize` (Number; _optional_)
    

Controls the disk size (in megabytes) of the VM used to unpack the image.

_Default value:_ 1024.

`name` (String; _optional_)
    

The name used for the output in the Nix store path.

_Default value:_ the value of `fromImage.name`.

#### Examples 

**Example 362. Exporting a Docker image with`dockerTools.exportImage`**

This example first builds a layered image with [`dockerTools.buildLayeredImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildLayeredImage> "buildLayeredImage"), and then exports its filesystem with `dockerTools.exportImage`.
    
    
    { dockerTools, hello }:
    dockerTools.exportImage {
      name = "hello";
      fromImage = dockerTools.buildLayeredImage {
        name = "hello";
        contents = [ hello ];
      };
    }
    

When building the package above, we can see the layers of the Docker image being unpacked to produce the final output:
    
    
    $ nix-build
    (some output removed for clarity)
    Unpacking base image...
    From-image name or tag wasn't set. Reading the first ID.
    Unpacking layer 5731199219418f175d1580dbca05677e69144425b2d9ecb60f416cd57ca3ca42/layer.tar
    tar: Removing leading `/' from member names
    Unpacking layer e2897bf34bb78c4a65736510204282d9f7ca258ba048c183d665bd0f3d24c5ec/layer.tar
    tar: Removing leading `/' from member names
    Unpacking layer 420aa5876dca4128cd5256da7dea0948e30ef5971712f82601718cdb0a6b4cda/layer.tar
    tar: Removing leading `/' from member names
    Unpacking layer ea5f4e620e7906c8ecbc506b5e6f46420e68d4b842c3303260d5eb621b5942e5/layer.tar
    tar: Removing leading `/' from member names
    Unpacking layer 65807b9abe8ab753fa97da8fb74a21fcd4725cc51e1b679c7973c97acd47ebcf/layer.tar
    tar: Removing leading `/' from member names
    Unpacking layer b7da2076b60ebc0ea6824ef641978332b8ac908d47b2d07ff31b9cc362245605/layer.tar
    Executing post-mount steps...
    Packing raw image...
    [    1.660036] reboot: Power down
    /nix/store/x6a5m7c6zdpqz1d8j7cnzpx9glzzvd2h-hello
    

The following command lists some of the contents of the output to verify that the structure of the archive is as expected:
    
    
    $ tar --exclude '*/share/*' --exclude 'nix/store/*/*' -tvf /nix/store/x6a5m7c6zdpqz1d8j7cnzpx9glzzvd2h-hello
    drwxr-xr-x root/0            0 1979-12-31 16:00 ./
    drwxr-xr-x root/0            0 1979-12-31 16:00 ./bin/
    lrwxrwxrwx root/0            0 1979-12-31 16:00 ./bin/hello -> /nix/store/h92a9jd0lhhniv2q417hpwszd4jhys7q-hello-2.12.1/bin/hello
    dr-xr-xr-x root/0            0 1979-12-31 16:00 ./nix/
    dr-xr-xr-x root/0            0 1979-12-31 16:00 ./nix/store/
    dr-xr-xr-x root/0            0 1979-12-31 16:00 ./nix/store/05zbwhz8a7i2v79r9j21pl6m6cj0xi8k-libunistring-1.1/
    dr-xr-xr-x root/0            0 1979-12-31 16:00 ./nix/store/ayg5rhjhi9ic73hqw33mjqjxwv59ndym-xgcc-13.2.0-libgcc/
    dr-xr-xr-x root/0            0 1979-12-31 16:00 ./nix/store/h92a9jd0lhhniv2q417hpwszd4jhys7q-hello-2.12.1/
    dr-xr-xr-x root/0            0 1979-12-31 16:00 ./nix/store/m59xdgkgnjbk8kk6k6vbxmqnf82mk9s0-libidn2-2.3.4/
    dr-xr-xr-x root/0            0 1979-12-31 16:00 ./nix/store/p3jshbwxiwifm1py0yq544fmdyy98j8a-glibc-2.38-27/
    drwxr-xr-x root/0            0 1979-12-31 16:00 ./share/
    

  

**Example 363. Importing an archive built with`dockerTools.exportImage` in Docker**

We will use the same package from [Example 362](</manual/nixpkgs/stable/#ex-dockerTools-exportImage-hello> "Example 362. Exporting a Docker image with dockerTools.exportImage") and import it into Docker.
    
    
    { dockerTools, hello }:
    dockerTools.exportImage {
      name = "hello";
      fromImage = dockerTools.buildLayeredImage {
        name = "hello";
        contents = [ hello ];
      };
    }
    

Building and importing it into Docker:
    
    
    $ nix-build
    (output removed for clarity)
    /nix/store/x6a5m7c6zdpqz1d8j7cnzpx9glzzvd2h-hello
    $ docker image import /nix/store/x6a5m7c6zdpqz1d8j7cnzpx9glzzvd2h-hello
    sha256:1d42dba415e9b298ea0decf6497fbce954de9b4fcb2984f91e307c8fedc1f52f
    $ docker image ls
    REPOSITORY                              TAG                IMAGE ID       CREATED         SIZE
    <none>                                  <none>             1d42dba415e9   4 seconds ago   32.6MB
    

  

**Example 364. Exploring output naming with`dockerTools.exportImage`**

`exportImage` does not require a `name` attribute if `fromImage` is a derivation, which means that the following works:
    
    
    { dockerTools, hello }:
    dockerTools.exportImage {
      fromImage = dockerTools.buildLayeredImage {
        name = "hello";
        contents = [ hello ];
      };
    }
    

However, since [`dockerTools.buildLayeredImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildLayeredImage> "buildLayeredImage")’s output ends with `.tar.gz`, the output of `exportImage` will also end with `.tar.gz`, even though the archive created with `exportImage` is uncompressed:
    
    
    $ nix-build
    (output removed for clarity)
    /nix/store/by3f40xvc4l6bkis74l0fj4zsy0djgkn-hello.tar.gz
    $ file /nix/store/by3f40xvc4l6bkis74l0fj4zsy0djgkn-hello.tar.gz
    /nix/store/by3f40xvc4l6bkis74l0fj4zsy0djgkn-hello.tar.gz: POSIX tar archive (GNU)
    

If the archive was actually compressed, the output of `file` would’ve mentioned that fact. Because of this, it may be important to set a proper `name` attribute when using `exportImage` with other functions from `dockerTools`.

  

**Example 365. Using`dockerTools.exportImage` with a path as `fromImage`**

It is possible to use a path as the value of the `fromImage` attribute when calling `dockerTools.exportImage`. However, when doing so, a `name` attribute **MUST** be specified, or you’ll encounter an error when evaluating the Nix code.

For this example, we’ll assume a Docker tarball image named `image.tar.gz` exists in the same directory where our package is defined:
    
    
    { dockerTools }:
    dockerTools.exportImage {
      name = "filesystem.tar";
      fromImage = ./image.tar.gz;
    }
    

Building this will give us the expected output:
    
    
    $ nix-build
    (output removed for clarity)
    /nix/store/w13l8h3nlkg0zv56k7rj0ai0l2zlf7ss-filesystem.tar
    

If you don’t specify a `name` attribute, you’ll encounter an evaluation error and the package won’t build.

  

### Environment Helpers 

When building Docker images with Nix, you might also want to add certain files that are expected to be available globally by the software you’re packaging. Simple examples are the `env` utility in `/usr/bin/env`, or trusted root TLS/SSL certificates. Such files will most likely not be included if you’re building a Docker image from scratch with Nix, and they might also not be included if you’re starting from a Docker image that doesn’t include them. The helpers in this section are packages that provide some of these commonly-needed global files.

Most of these helpers are packages, which means you have to add them to the list of contents to be included in the image (this changes depending on the function you’re using to build the image). [Example 366](</manual/nixpkgs/stable/#ex-dockerTools-helpers-buildImage> "Example 366. Using dockerTools’s environment helpers with buildImage") and [Example 367](</manual/nixpkgs/stable/#ex-dockerTools-helpers-buildLayeredImage> "Example 367. Using dockerTools’s environment helpers with buildLayeredImage") show how to include these packages on `dockerTools` functions that build an image. For more details on how that works, see the documentation for the function you’re using.

#### usrBinEnv 

This provides the `env` utility at `/usr/bin/env`. This is currently implemented by linking to the `env` binary from the `coreutils` package, but is considered an implementation detail that could change in the future.

#### binSh 

This provides a `/bin/sh` link to the `bash` binary from the `bash` package. Because of this, it supports cases such as running a command interactively in a container (for example by running `docker container run -it <image_name>`).

#### caCertificates 

This adds trusted root TLS/SSL certificates from the `cacert` package in multiple locations in an attempt to be compatible with binaries built for multiple Linux distributions. The locations currently used are:

  * `/etc/ssl/certs/ca-bundle.crt`

  * `/etc/ssl/certs/ca-certificates.crt`

  * `/etc/pki/tls/certs/ca-bundle.crt`

#### fakeNss 

This is a re-export of the `fakeNss` package from Nixpkgs. See [the section called “fakeNss”](</manual/nixpkgs/stable/#sec-fakeNss> "fakeNss").

#### shadowSetup 

This is a string containing a script that sets up files needed for [`shadow`](<https://github.com/shadow-maint/shadow>) to work (using the `shadow` package from Nixpkgs), and alters `PATH` to make all its utilities available in the same script. It is intended to be used with other dockerTools functions in attributes that expect scripts. After the script in `shadowSetup` runs, you’ll then be able to add more commands that make use of the utilities in `shadow`, such as adding any extra users and/or groups. See [Example 368](</manual/nixpkgs/stable/#ex-dockerTools-shadowSetup-buildImage> "Example 368. Using dockerTools.shadowSetup with dockerTools.buildImage") and [Example 369](</manual/nixpkgs/stable/#ex-dockerTools-shadowSetup-buildLayeredImage> "Example 369. Using dockerTools.shadowSetup with dockerTools.buildLayeredImage") to better understand how to use it.

`shadowSetup` achieves a result similar to [`fakeNss`](</manual/nixpkgs/stable/#sssec-pkgs-dockerTools-helpers-fakeNss> "fakeNss"), but only sets up a `root` user with different values for the home directory and the shell to use, in addition to setting up files for [PAM](<https://en.wikipedia.org/wiki/Linux_PAM>) and a [login.defs(5)](<https://man.archlinux.org/man/login.defs.5>) file.

### Caution

Using both `fakeNss` and `shadowSetup` at the same time will either cause your build to break or produce unexpected results. Use either `fakeNss` or `shadowSetup` depending on your use case, but avoid using both.

### Note

When used with [`buildLayeredImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildLayeredImage> "buildLayeredImage") or [`streamLayeredImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-streamLayeredImage> "streamLayeredImage"), you will have to set the `enableFakechroot` attribute to `true`, or else the script in `shadowSetup` won’t run properly. See [Example 369](</manual/nixpkgs/stable/#ex-dockerTools-shadowSetup-buildLayeredImage> "Example 369. Using dockerTools.shadowSetup with dockerTools.buildLayeredImage").

#### Examples 

**Example 366. Using`dockerTools`’s environment helpers with `buildImage`**

This example adds the [`binSh`](</manual/nixpkgs/stable/#sssec-pkgs-dockerTools-helpers-binSh> "binSh") helper to a basic Docker image built with [`dockerTools.buildImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildImage> "buildImage"). This helper makes it possible to enter a shell inside the container. This is the `buildImage` equivalent of [Example 367](</manual/nixpkgs/stable/#ex-dockerTools-helpers-buildLayeredImage> "Example 367. Using dockerTools’s environment helpers with buildLayeredImage").
    
    
    { dockerTools, hello }:
    dockerTools.buildImage {
      name = "env-helpers";
      tag = "latest";
    
      copyToRoot = [
        hello
        dockerTools.binSh
      ];
    }
    

After building the image and loading it in Docker, we can create a container based on it and enter a shell inside the container. This is made possible by `binSh`.
    
    
    $ nix-build
    (some output removed for clarity)
    /nix/store/2p0i3i04cgjlk71hsn7ll4kxaxxiv4qg-docker-image-env-helpers.tar.gz
    $ docker image load -i /nix/store/2p0i3i04cgjlk71hsn7ll4kxaxxiv4qg-docker-image-env-helpers.tar.gz
    (output removed for clarity)
    $ docker container run --rm -it env-helpers:latest /bin/sh
    sh-5.2# help
    GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu)
    (rest of output removed for clarity)
    

  

**Example 367. Using`dockerTools`’s environment helpers with `buildLayeredImage`**

This example adds the [`binSh`](</manual/nixpkgs/stable/#sssec-pkgs-dockerTools-helpers-binSh> "binSh") helper to a basic Docker image built with [`dockerTools.buildLayeredImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-buildLayeredImage> "buildLayeredImage"). This helper makes it possible to enter a shell inside the container. This is the `buildLayeredImage` equivalent of [Example 366](</manual/nixpkgs/stable/#ex-dockerTools-helpers-buildImage> "Example 366. Using dockerTools’s environment helpers with buildImage").
    
    
    { dockerTools, hello }:
    dockerTools.buildLayeredImage {
      name = "env-helpers";
      tag = "latest";
    
      contents = [
        hello
        dockerTools.binSh
      ];
    
      config = {
        Cmd = [ "/bin/hello" ];
      };
    }
    

After building the image and loading it in Docker, we can create a container based on it and enter a shell inside the container. This is made possible by `binSh`.
    
    
    $ nix-build
    (some output removed for clarity)
    /nix/store/rpf47f4z5b9qr4db4ach9yr4b85hjhxq-env-helpers.tar.gz
    $ docker image load -i /nix/store/rpf47f4z5b9qr4db4ach9yr4b85hjhxq-env-helpers.tar.gz
    (output removed for clarity)
    $ docker container run --rm -it env-helpers:latest /bin/sh
    sh-5.2# help
    GNU bash, version 5.2.21(1)-release (x86_64-pc-linux-gnu)
    (rest of output removed for clarity)
    

  

**Example 368. Using`dockerTools.shadowSetup` with `dockerTools.buildImage`**

This is an example that shows how to use `shadowSetup` with `dockerTools.buildImage`. Note that the extra script in `runAsRoot` uses `groupadd` and `useradd`, which are binaries provided by the `shadow` package. These binaries are added to the `PATH` by the `shadowSetup` script, but only for the duration of `runAsRoot`.
    
    
    { dockerTools, hello }:
    dockerTools.buildImage {
      name = "shadow-basic";
      tag = "latest";
    
      copyToRoot = [ hello ];
    
      runAsRoot = ''
        ${dockerTools.shadowSetup}
        groupadd -r hello
        useradd -r -g hello hello
        mkdir /data
        chown hello:hello /data
      '';
    
      config = {
        Cmd = [ "/bin/hello" ];
        WorkingDir = "/data";
      };
    }
    

  

**Example 369. Using`dockerTools.shadowSetup` with `dockerTools.buildLayeredImage`**

It accomplishes the same thing as [Example 368](</manual/nixpkgs/stable/#ex-dockerTools-shadowSetup-buildImage> "Example 368. Using dockerTools.shadowSetup with dockerTools.buildImage"), but using `buildLayeredImage` instead.

Note that the extra script in `fakeRootCommands` uses `groupadd` and `useradd`, which are binaries provided by the `shadow` package. These binaries are added to the `PATH` by the `shadowSetup` script, but only for the duration of `fakeRootCommands`.
    
    
    { dockerTools, hello }:
    dockerTools.buildLayeredImage {
      name = "shadow-basic";
      tag = "latest";
    
      contents = [ hello ];
    
      fakeRootCommands = ''
        ${dockerTools.shadowSetup}
        groupadd -r hello
        useradd -r -g hello hello
        mkdir /data
        chown hello:hello /data
      '';
      enableFakechroot = true;
    
      config = {
        Cmd = [ "/bin/hello" ];
        WorkingDir = "/data";
      };
    }
    

  

### buildNixShellImage 

`buildNixShellImage` uses [`streamNixShellImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-streamNixShellImage> "streamNixShellImage") underneath to build a compressed Docker-compatible repository tarball of an image that sets up an environment similar to that of running `nix-shell` on a derivation. Basically, `buildNixShellImage` runs the script created by `streamNixShellImage` to save the compressed image in the Nix store.

`buildNixShellImage` supports the same options as `streamNixShellImage`, see [`streamNixShellImage`](</manual/nixpkgs/stable/#ssec-pkgs-dockerTools-streamNixShellImage> "streamNixShellImage") for details.

#### Examples 

**Example 370. Building a Docker image with`buildNixShellImage` with the build environment for the `hello` package**

This example shows how to build the `hello` package inside a Docker container built with `buildNixShellImage`. The Docker image generated will have a name like `hello-<version>-env` and tag `latest`. This example is the `buildNixShellImage` equivalent of [Example 371](</manual/nixpkgs/stable/#ex-dockerTools-streamNixShellImage-hello> "Example 371. Building a Docker image with streamNixShellImage with the build environment for the hello package").
    
    
    { dockerTools, hello }:
    dockerTools.buildNixShellImage {
      drv = hello;
      tag = "latest";
    }
    

The result of building this package is a `.tar.gz` file that can be loaded into Docker:
    
    
    $ nix-build
    (some output removed for clarity)
    /nix/store/pkj1sgzaz31wl0pbvbg3yp5b3kxndqms-hello-2.12.1-env.tar.gz
    
    $ docker image load -i /nix/store/pkj1sgzaz31wl0pbvbg3yp5b3kxndqms-hello-2.12.1-env.tar.gz
    (some output removed for clarity)
    Loaded image: hello-2.12.1-env:latest
    

After starting an interactive container, the derivation can be built by running `buildDerivation`, and the output can be executed as expected:
    
    
    $ docker container run -it hello-2.12.1-env:latest
    [nix-shell:~]$ buildDerivation
    Running phase: unpackPhase
    unpacking source archive /nix/store/pa10z4ngm0g83kx9mssrqzz30s84vq7k-hello-2.12.1.tar.gz
    source root is hello-2.12.1
    (some output removed for clarity)
    Running phase: fixupPhase
    shrinking RPATHs of ELF executables and libraries in /nix/store/f2vs29jibd7lwxyj35r9h87h6brgdysz-hello-2.12.1
    shrinking /nix/store/f2vs29jibd7lwxyj35r9h87h6brgdysz-hello-2.12.1/bin/hello
    checking for references to /build/ in /nix/store/f2vs29jibd7lwxyj35r9h87h6brgdysz-hello-2.12.1...
    gzipping man pages under /nix/store/f2vs29jibd7lwxyj35r9h87h6brgdysz-hello-2.12.1/share/man/
    patching script interpreter paths in /nix/store/f2vs29jibd7lwxyj35r9h87h6brgdysz-hello-2.12.1
    stripping (with command strip and flags -S -p) in  /nix/store/f2vs29jibd7lwxyj35r9h87h6brgdysz-hello-2.12.1/bin
    
    [nix-shell:~]$ $out/bin/hello
    Hello, world!
    

  

### streamNixShellImage 

`streamNixShellImage` builds a **script** which, when run, will stream to stdout a Docker-compatible repository tarball of an image that sets up an environment similar to that of running `nix-shell` on a derivation. This means that `streamNixShellImage` does not output an image into the Nix store, but only a script that builds the image, saving on IO and disk/cache space, particularly with large images. See [Example 371](</manual/nixpkgs/stable/#ex-dockerTools-streamNixShellImage-hello> "Example 371. Building a Docker image with streamNixShellImage with the build environment for the hello package") to understand how to load in Docker the image generated by this script.

The environment set up by `streamNixShellImage` somewhat resembles the Nix sandbox typically used by `nix-build`, with a major difference being that access to the internet is allowed. It also behaves like an interactive `nix-shell`, running things like `shellHook` (see [Example 373](</manual/nixpkgs/stable/#ex-dockerTools-streamNixShellImage-addingShellHook> "Example 373. Adding a shellHook to a Docker image built with streamNixShellImage")) and setting an interactive prompt. If the derivation is buildable (i.e. `nix-build` can be used on it), running `buildDerivation` in the container will build the derivation, with all its outputs being available in the correct `/nix/store` paths, pointed to by the respective environment variables (e.g. `$out`).

### Caution

The environment in the image doesn’t match `nix-shell` or `nix-build` exactly, and this function is known not to work correctly for fixed-output derivations, content-addressed derivations, impure derivations and other special types of derivations.

#### Inputs 

`streamNixShellImage` expects one argument with the following attributes:

`drv` (Attribute Set)
    

The derivation for which the environment in the image will be set up. Adding packages to the Docker image is possible by extending the list of `nativeBuildInputs` of this derivation. See [Example 372](</manual/nixpkgs/stable/#ex-dockerTools-streamNixShellImage-extendingBuildInputs> "Example 372. Adding extra packages to a Docker image built with streamNixShellImage") for how to do that. Similarly, you can extend the image initialization script by extending `shellHook`. [Example 373](</manual/nixpkgs/stable/#ex-dockerTools-streamNixShellImage-addingShellHook> "Example 373. Adding a shellHook to a Docker image built with streamNixShellImage") shows how to do that.

`name` (String; _optional_)
    

The name of the generated image.

_Default value:_ the value of `drv.name + "-env"`.

`tag` (String or Null; _optional_)
    

Tag of the generated image. If `null`, the hash of the nix derivation that builds the Docker image will be used as the tag.

_Default value:_ `null`.

`uid` (Number; _optional_)
    

The user ID to run the container as. This can be seen as a `nixbld` build user.

_Default value:_ 1000.

`gid` (Number; _optional_)
    

The group ID to run the container as. This can be seen as a `nixbld` build group.

_Default value:_ 1000.

`homeDirectory` (String; _optional_)
    

The home directory of the user the container is running as.

_Default value:_ `/build`.

`shell` (String; _optional_)
    

The path to the `bash` binary to use as the shell. This shell is started when running the image. This can be seen as an equivalent of the `NIX_BUILD_SHELL` [environment variable](<https://nixos.org/manual/nix/stable/command-ref/nix-shell.html#environment-variables>) for [nix-shell(1)](<https://nixos.org/manual/nix/stable/command-ref/nix-shell.html>).

_Default value:_ the `bash` binary from the `bash` package.

`command` (String or Null; _optional_)
    

If specified, this command will be run in the environment of the derivation in an interactive shell. A call to `exit` will be added after the command if it is specified, so the shell will exit after it’s finished running. This can be seen as an equivalent of the `--command` option in [nix-shell(1)](<https://nixos.org/manual/nix/stable/command-ref/nix-shell.html>).

_Default value:_ `null`.

`run` (String or Null; _optional_)
    

Similar to the `command` attribute, but runs the command in a non-interactive shell instead. A call to `exit` will be added after the command if it is specified, so the shell will exit after it’s finished running. This can be seen as an equivalent of the `--run` option in [nix-shell(1)](<https://nixos.org/manual/nix/stable/command-ref/nix-shell.html>).

_Default value:_ `null`.

#### Examples 

**Example 371. Building a Docker image with`streamNixShellImage` with the build environment for the `hello` package**

This example shows how to build the `hello` package inside a Docker container built with `streamNixShellImage`. The Docker image generated will have a name like `hello-<version>-env` and tag `latest`. This example is the `streamNixShellImage` equivalent of [Example 370](</manual/nixpkgs/stable/#ex-dockerTools-buildNixShellImage-hello> "Example 370. Building a Docker image with buildNixShellImage with the build environment for the hello package").
    
    
    { dockerTools, hello }:
    dockerTools.streamNixShellImage {
      drv = hello;
      tag = "latest";
    }
    

The result of building this package is a script. Running this script and piping it into `docker image load` gives you the same image that was built in [Example 370](</manual/nixpkgs/stable/#ex-dockerTools-buildNixShellImage-hello> "Example 370. Building a Docker image with buildNixShellImage with the build environment for the hello package").
    
    
    $ nix-build
    (some output removed for clarity)
    /nix/store/8vhznpz2frqazxnd8pgdvf38jscdypax-stream-hello-2.12.1-env
    
    $ /nix/store/8vhznpz2frqazxnd8pgdvf38jscdypax-stream-hello-2.12.1-env | docker image load
    (some output removed for clarity)
    Loaded image: hello-2.12.1-env:latest
    

After starting an interactive container, the derivation can be built by running `buildDerivation`, and the output can be executed as expected:
    
    
    $ docker container run -it hello-2.12.1-env:latest
    [nix-shell:~]$ buildDerivation
    Running phase: unpackPhase
    unpacking source archive /nix/store/pa10z4ngm0g83kx9mssrqzz30s84vq7k-hello-2.12.1.tar.gz
    source root is hello-2.12.1
    (some output removed for clarity)
    Running phase: fixupPhase
    shrinking RPATHs of ELF executables and libraries in /nix/store/f2vs29jibd7lwxyj35r9h87h6brgdysz-hello-2.12.1
    shrinking /nix/store/f2vs29jibd7lwxyj35r9h87h6brgdysz-hello-2.12.1/bin/hello
    checking for references to /build/ in /nix/store/f2vs29jibd7lwxyj35r9h87h6brgdysz-hello-2.12.1...
    gzipping man pages under /nix/store/f2vs29jibd7lwxyj35r9h87h6brgdysz-hello-2.12.1/share/man/
    patching script interpreter paths in /nix/store/f2vs29jibd7lwxyj35r9h87h6brgdysz-hello-2.12.1
    stripping (with command strip and flags -S -p) in  /nix/store/f2vs29jibd7lwxyj35r9h87h6brgdysz-hello-2.12.1/bin
    
    [nix-shell:~]$ $out/bin/hello
    Hello, world!
    

  

**Example 372. Adding extra packages to a Docker image built with`streamNixShellImage`**

This example shows how to add extra packages to an image built with `streamNixShellImage`. In this case, we’ll add the `cowsay` package. The Docker image generated will have a name like `hello-<version>-env` and tag `latest`. This example uses [Example 371](</manual/nixpkgs/stable/#ex-dockerTools-streamNixShellImage-hello> "Example 371. Building a Docker image with streamNixShellImage with the build environment for the hello package") as a starting point.
    
    
    {
      dockerTools,
      cowsay,
      hello,
    }:
    dockerTools.streamNixShellImage {
      tag = "latest";
      drv = hello.overrideAttrs (old: {
        nativeBuildInputs = old.nativeBuildInputs or [ ] ++ [ cowsay ];
      });
    }
    

The result of building this package is a script which can be run and piped into `docker image load` to load the generated image.
    
    
    $ nix-build
    (some output removed for clarity)
    /nix/store/h5abh0vljgzg381lna922gqknx6yc0v7-stream-hello-2.12.1-env
    
    $ /nix/store/h5abh0vljgzg381lna922gqknx6yc0v7-stream-hello-2.12.1-env | docker image load
    (some output removed for clarity)
    Loaded image: hello-2.12.1-env:latest
    

After starting an interactive container, we can verify the extra package is available by running `cowsay`:
    
    
    $ docker container run -it hello-2.12.1-env:latest
    [nix-shell:~]$ cowsay "Hello, world!"
     _______________
    < Hello, world! >
     ---------------
            \   ^__^
             \  (oo)\_______
                (__)\       )\/\
                    ||----w |
                    ||     ||
    

  

**Example 373. Adding a`shellHook` to a Docker image built with `streamNixShellImage`**

This example shows how to add a `shellHook` command to an image built with `streamNixShellImage`. In this case, we’ll simply output the string `Hello, world!`. The Docker image generated will have a name like `hello-<version>-env` and tag `latest`. This example uses [Example 371](</manual/nixpkgs/stable/#ex-dockerTools-streamNixShellImage-hello> "Example 371. Building a Docker image with streamNixShellImage with the build environment for the hello package") as a starting point.
    
    
    { dockerTools, hello }:
    dockerTools.streamNixShellImage {
      tag = "latest";
      drv = hello.overrideAttrs (old: {
        shellHook = ''
          ${old.shellHook or ""}
          echo "Hello, world!"
        '';
      });
    }
    

The result of building this package is a script which can be run and piped into `docker image load` to load the generated image.
    
    
    $ nix-build
    (some output removed for clarity)
    /nix/store/iz4dhdvgzazl5vrgyz719iwjzjy6xlx1-stream-hello-2.12.1-env
    
    $ /nix/store/iz4dhdvgzazl5vrgyz719iwjzjy6xlx1-stream-hello-2.12.1-env | docker image load
    (some output removed for clarity)
    Loaded image: hello-2.12.1-env:latest
    

After starting an interactive container, we can see the result of the `shellHook`:
    
    
    $ docker container run -it hello-2.12.1-env:latest
    Hello, world!
    
    [nix-shell:~]$
