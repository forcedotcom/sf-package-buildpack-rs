/**
 * Created by mhoefer on 2/8/21.
 */

@IsTest
public class SomeCallableTest {
    @IsTest
    static void testCallable() {
        Callable c = (Callable) Type.forName(SomeCallable.class.getName()).newInstance();
        try {
            c.call('foo', new Map<String, Object>{'bar' => 'cliche'});
            System.assert(false, 'Should have thrown an exception');
        } catch (Exception e) {
            // Good
        }
    }
}
