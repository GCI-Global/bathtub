# Bathtub - GRBL Gantry Control for Humans
> Bathtub has been used in production to manufacture thousands of dollars of product.
> Works on Windows and Linux (Tested on both Fedora and Pop_OS!)!
Most GRBL automation is done with G-Code scripts. Long text files that need to be manually edited with XYZ coordinates that are easy to mistake.
The goal of Bathtub was to simplify that prossess to be more like a consumer printer, mostly easy to use :smile:.
Bathtub does this my making all of the complex proccess at setup and then daily use is very simple.
![](https://github.com/GCI-Global/bathtub/blob/readme_update/img/manual.png?raw=true)

## Setup
Bathtub requires that the XYZ coordinate of each destination (node), and then possible actions at each destination (i.e. Pick something up, wait, etc.).
Luckily though this can all be done in a convenient interface that automatically prevents physical imposibilities and logical errors.
![](https://github.com/GCI-Global/bathtub/blob/readme_update/img/nodes.png?raw=true)
![](https://github.com/GCI-Global/bathtub/blob/readme_update/img/actions.png?raw=true)

## Run
###Now to the easy part!
Bathtub has two main modes `Manual` and `Run`. Because the destinations and relationships to nearby nodes were set in the setup Bathtub is able to
build safe paths between any given nodes. Bathtub can generate a safe path from its current position to any destination.

## Manual
To manually sent the gantry to a position there is the manual view, just click any destination on the grid (The grid is generated based on the node configuration. All buttons are relative to the coresponding destination in real life.) and Bathtub will control the gantry to its final destination, and update the UI displaying the current location. It is possible to pause and/or cancel at any time in the navagation.

## Run
Run is as simple as click run! Select what recipe you would like to run and Bathtub will control the gantry across the whole process updating the user with a list displaying past, current, and future steps.

![](https://github.com/GCI-Global/bathtub/blob/readme_update/img/run.png?raw=true)

## Build
Build is great too! Rather than having to scroll through long G-Code scripts copy/paste ect., This is a simple list that can be reordered, modified, saved, deleted all in plain english, no need to get confused over XYZ coordinates.
![](https://github.com/GCI-Global/bathtub/blob/readme_update/img/build.png?raw=true)

## Logs
Bathtub keeps detailed logs of all actions taken within the application. Time, what actions, operating system user are all saved. This is great for debugging, butalso if Bathtub is used in an environment has multiple technicains, it is possible to keep track of who did what to monitor for user error.
![](https://github.com/GCI-Global/bathtub/blob/readme_update/img/logs.png?raw=true)

## Search
Search is an extransion of the logs. Simply search for any log that is either named or contains any text or multiple strings of text. This seach is multithreaded and has been tested on on old buisness laptop to search 20,000 logs in 10 seconds. This is likely good enough to keep detailed logs for years and find any result almost immediatly.
![](https://github.com/GCI-Global/bathtub/blob/readme_update/img/search.png?raw=true)

## Misc
Bathtub automatically connects to COM ports, there is no need for users to understand what those are. Just plug and play, like a printer!
![](https://github.com/GCI-Global/bathtub/blob/readme_update/img/connect.png?raw=true)
Bathtub detects and notify's of errors all across the application, here are just a few examples
![](https://github.com/GCI-Global/bathtub/blob/readme_update/img/errors1.png?raw=true)
![](https://github.com/GCI-Global/bathtub/blob/readme_update/img/errors2.png?raw=true)


## Any Questions?
If you have any questions / issues please make a GitHub issue, and I will try to help!

> Bathtub is proudly written 100% in Rust and uses [Iced](https://github.com/hecrj/iced) as the UI framework.
