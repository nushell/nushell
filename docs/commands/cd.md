# cd

If you didn't already know, the `cd` command is very simple. It stands for 'change directory' and it does exactly that. It changes the current directory to the one specified. If no directory is specified, it takes you to the home directory. Additionally, using `cd ..` takes you to the parent directory.

## Examples

```shell
/home/username> cd Desktop
/home/username/Desktop> now your current directory has been changed
```

```shell
/home/username/Desktop/nested/folders> cd ..
/home/username/Desktop/nested> cd ..
/home/username/Desktop> cd ../Documents/school_related
/home/username/Documents/school_related> cd ../../..
/home/>
```

```shell
/home/username/Desktop/super/duper/crazy/nested/folders> cd
/home/username> cd ../../usr
/usr> cd
/home/username>
```

Using `cd -` will take you to the previous directory:

```shell
/home/username/Desktop/super/duper/crazy/nested/folders> cd
/home/username> cd -
/home/username/Desktop/super/duper/crazy/nested/folders> cd
```
