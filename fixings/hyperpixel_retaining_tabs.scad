$fn=100;

//The bar clip that holds the screen in place.
difference() {
    union() {
    translate([3.5,0,1]) cube ([12,70,2],center=true);
    translate([6,0,1.99]) cube([7,55,2],center=true);
    }
    translate([0,59/2 +2, -25]) cylinder(d=2, h= 50);
    translate([0, -59/2 -2, -25]) cylinder(d=2, h= 50);
}

//Round clip
translate([20,10,0]) difference() {
    union() {
        cylinder(d=10,h=2);
        translate([5.8,-3.4,1.5/2 + 1.99]) cube([8,4,1.5],center=true);   
    }    
    cylinder(d=2,h=50);
}


//Round clip (mirror image)
translate([20,-10,0]) difference() {
    union() {
        cylinder(d=10,h=2);
        translate([5.8,3.4,1.5/2 + 1.99]) cube([8,4,1.5],center=true);   
    }    
    cylinder(d=2,h=50);
}