#import <AppKit/AppKit.h>
#import <CoreServices/CoreServices.h>

@interface MotionCueLinkDelegate : NSObject <NSApplicationDelegate>
@end
@implementation MotionCueLinkDelegate
- (void)application:(NSApplication *)application openURLs:(NSArray<NSURL *> *)urls {
    NSURL *resources = NSBundle.mainBundle.bundleURL.URLByDeletingLastPathComponent;
    NSURL *executable = [resources.URLByDeletingLastPathComponent URLByAppendingPathComponent:@"MacOS/motion-cue"];
    for (NSURL *url in urls) {
        if (![url.scheme.lowercaseString isEqualToString:@"motioncue"] || ![url.host isEqualToString:@"play"]) {
            NSLog(@"MotionCue rejected unsupported link");
            continue;
        }
        NSTask *task = [NSTask new];
        task.executableURL = executable;
        task.arguments = @[url.absoluteString];
        task.standardInput = [NSFileHandle fileHandleWithNullDevice];
        task.standardOutput = [NSFileHandle fileHandleWithNullDevice];
        task.standardError = [NSFileHandle fileHandleWithNullDevice];
        task.terminationHandler = ^(NSTask *finished) {
            if (finished.terminationStatus != 0) {
                NSLog(@"MotionCue link forwarding exited with status %d", finished.terminationStatus);
            }
        };
        NSError *error = nil;
        if (![task launchAndReturnError:&error]) {
            NSLog(@"MotionCue link forwarding failed: %@", error.localizedDescription);
        }
    }
}
@end

int main(void) {
    @autoreleasepool {
        NSApplication *application = NSApplication.sharedApplication;
        [application setActivationPolicy:NSApplicationActivationPolicyProhibited];
        LSSetDefaultHandlerForURLScheme(CFSTR("motioncue"), CFSTR("com.motioncue.desktop.link"));
        MotionCueLinkDelegate *delegate = [MotionCueLinkDelegate new];
        application.delegate = delegate;
        [application run];
    }
    return 0;
}
